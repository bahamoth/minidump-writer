use {
    super::{
        auxv::AuxvError,
        errors::{AndroidError, MapsReaderError},
        serializers::*,
    },
    crate::{
        linux::{
            auxv::AuxvDumpInfo,
            errors::{DumperError, ThreadInfoError},
            maps_reader::MappingInfo,
            module_reader,
            thread_info::ThreadInfo,
            Pid,
        },
        serializers::*,
    },
    error_graph::{ErrorList, WriteErrorList},
    failspot::failspot,
    nix::{
        errno::Errno,
        sys::{ptrace, signal, wait},
    },
    procfs_core::{
        process::{MMPermissions, ProcState, Stat},
        FromRead, ProcError,
    },
    std::{
        ffi::OsString,
        path,
        result::Result,
        time::{Duration, Instant},
    },
    thiserror::Error,
};

#[cfg(target_os = "android")]
use crate::linux::android::late_process_mappings;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use crate::thread_info;

#[derive(Debug, Clone)]
pub struct Thread {
    pub tid: Pid,
    pub name: Option<String>,
}

#[derive(Debug)]
pub struct PtraceDumper {
    pub pid: Pid,
    threads_suspended: bool,
    pub threads: Vec<Thread>,
    pub auxv: AuxvDumpInfo,
    pub mappings: Vec<MappingInfo>,
    pub page_size: usize,
}

#[cfg(target_pointer_width = "32")]
pub const AT_SYSINFO_EHDR: u32 = 33;
#[cfg(target_pointer_width = "64")]
pub const AT_SYSINFO_EHDR: u64 = 33;

impl Drop for PtraceDumper {
    fn drop(&mut self) {
        // Always try to resume all threads (e.g. in case of error)
        self.resume_threads(error_graph::strategy::DontCare);
        // Always allow the process to continue.
        let _ = self.continue_process();
    }
}

#[derive(Debug, Error, serde::Serialize)]
pub enum InitError {
    #[error("failed to read auxv")]
    ReadAuxvFailed(#[source] crate::auxv::AuxvError),
    #[error("IO error for file {0}")]
    IOError(
        String,
        #[source]
        #[serde(serialize_with = "serialize_io_error")]
        std::io::Error,
    ),
    #[error("Failed Android specific late init")]
    AndroidLateInitError(#[from] AndroidError),
    #[error("Failed to read the page size")]
    PageSizeError(
        #[from]
        #[serde(serialize_with = "serialize_nix_error")]
        nix::Error,
    ),
    #[error("Ptrace does not function within the same process")]
    CannotPtraceSameProcess,
    #[error("Failed to stop the target process")]
    StopProcessFailed(#[source] StopProcessError),
    #[error("Errors occurred while filling missing Auxv info")]
    FillMissingAuxvInfoErrors(#[source] ErrorList<AuxvError>),
    #[error("Failed filling missing Auxv info")]
    FillMissingAuxvInfoFailed(#[source] AuxvError),
    #[error("Failed reading proc/pid/task entry for process")]
    ReadProcessThreadEntryFailed(
        #[source]
        #[serde(serialize_with = "serialize_io_error")]
        std::io::Error,
    ),
    #[error("Process task entry `{0:?}` could not be parsed as a TID")]
    ProcessTaskEntryNotTid(OsString),
    #[error("Failed to read thread name")]
    ReadThreadNameFailed(
        #[source]
        #[serde(serialize_with = "serialize_io_error")]
        std::io::Error,
    ),
    #[error("Proc task directory `{0:?}` is not a directory")]
    ProcPidTaskNotDirectory(String),
    #[error("Errors while enumerating threads")]
    EnumerateThreadsErrors(#[source] ErrorList<InitError>),
    #[error("Failed to enumerate threads")]
    EnumerateThreadsFailed(#[source] Box<InitError>),
    #[error("Failed to read process map file")]
    ReadProcessMapFileFailed(
        #[source]
        #[serde(serialize_with = "serialize_proc_error")]
        ProcError,
    ),
    #[error("Failed to aggregate process mappings")]
    AggregateMappingsFailed(#[source] MapsReaderError),
    #[error("Failed to enumerate process mappings")]
    EnumerateMappingsFailed(#[source] Box<InitError>),
}

#[derive(Debug, thiserror::Error, serde::Serialize)]
pub enum StopProcessError {
    #[error("Failed to stop the process")]
    Stop(
        #[from]
        #[serde(serialize_with = "serialize_nix_error")]
        nix::Error,
    ),
    #[error("Failed to get the process state")]
    State(
        #[from]
        #[serde(serialize_with = "serialize_proc_error")]
        ProcError,
    ),
    #[error("Timeout waiting for process to stop")]
    Timeout,
}

#[derive(Debug, thiserror::Error)]
pub enum ContinueProcessError {
    #[error("Failed to continue the process")]
    Continue(#[from] Errno),
}

/// PTRACE_DETACH the given pid.
///
/// This handles special errno cases (ESRCH) which we won't consider errors.
fn ptrace_detach(child: Pid) -> Result<(), DumperError> {
    let pid = nix::unistd::Pid::from_raw(child);
    ptrace::detach(pid, None).or_else(|e| {
        // errno is set to ESRCH if the pid no longer exists, but we don't want to error in that
        // case.
        if e == nix::Error::ESRCH {
            Ok(())
        } else {
            Err(DumperError::PtraceDetachError(child, e))
        }
    })
}

impl PtraceDumper {
    /// Constructs a dumper for extracting information from the specified process id
    pub fn new_report_soft_errors(
        pid: Pid,
        stop_timeout: Duration,
        auxv: AuxvDumpInfo,
        soft_errors: impl WriteErrorList<InitError>,
    ) -> Result<Self, InitError> {
        if pid == std::process::id() as i32 {
            return Err(InitError::CannotPtraceSameProcess);
        }

        let mut dumper = Self {
            pid,
            threads_suspended: false,
            threads: Vec::new(),
            auxv,
            mappings: Vec::new(),
            page_size: 0,
        };
        dumper.init(stop_timeout, soft_errors)?;
        Ok(dumper)
    }

    // TODO: late_init for chromeos and android
    pub fn init(
        &mut self,
        stop_timeout: Duration,
        mut soft_errors: impl WriteErrorList<InitError>,
    ) -> Result<(), InitError> {
        // Stopping the process is best-effort.
        if let Err(e) = self.stop_process(stop_timeout) {
            soft_errors.push(InitError::StopProcessFailed(e));
        }

        // Even if we completely fail to fill in any additional Auxv info, we can still press
        // forward.
        if let Err(e) = self.auxv.try_filling_missing_info(
            self.pid,
            soft_errors.subwriter(InitError::FillMissingAuxvInfoErrors),
        ) {
            soft_errors.push(InitError::FillMissingAuxvInfoFailed(e));
        }

        // If we completely fail to enumerate any threads... Some information is still better than
        // no information!
        if let Err(e) =
            self.enumerate_threads(soft_errors.subwriter(InitError::EnumerateThreadsErrors))
        {
            soft_errors.push(InitError::EnumerateThreadsFailed(Box::new(e)));
        }

        // Same with mappings -- Some information is still better than no information!
        if let Err(e) = self.enumerate_mappings() {
            soft_errors.push(InitError::EnumerateMappingsFailed(Box::new(e)));
        }

        self.page_size = nix::unistd::sysconf(nix::unistd::SysconfVar::PAGE_SIZE)?
            .expect("page size apparently unlimited: doesn't make sense.")
            as usize;

        Ok(())
    }

    #[cfg_attr(not(target_os = "android"), allow(clippy::unused_self))]
    pub fn late_init(&mut self) -> Result<(), InitError> {
        #[cfg(target_os = "android")]
        {
            late_process_mappings(self.pid, &mut self.mappings)?;
        }
        Ok(())
    }

    /// Suspends a thread by attaching to it.
    pub fn suspend_thread(child: Pid) -> Result<(), DumperError> {
        use DumperError::PtraceAttachError as AttachErr;

        let pid = nix::unistd::Pid::from_raw(child);
        // This may fail if the thread has just died or debugged.
        ptrace::attach(pid).map_err(|e| AttachErr(child, e))?;
        loop {
            match wait::waitpid(pid, Some(wait::WaitPidFlag::__WALL)) {
                Ok(status) => {
                    let wait::WaitStatus::Stopped(_, status) = status else {
                        return Err(DumperError::WaitPidError(
                            child,
                            nix::errno::Errno::UnknownErrno,
                        ));
                    };

                    // Any signal will stop the thread, make sure it is SIGSTOP. Otherwise, this
                    // signal will be delivered after PTRACE_DETACH, and the thread will enter
                    // the "T (stopped)" state.
                    if status == nix::sys::signal::SIGSTOP {
                        break;
                    }

                    // Signals other than SIGSTOP that are received need to be reinjected,
                    // or they will otherwise get lost.
                    if let Err(err) = ptrace::cont(pid, status) {
                        return Err(DumperError::WaitPidError(child, err));
                    }
                }
                Err(Errno::EINTR) => continue,
                Err(e) => {
                    ptrace_detach(child)?;
                    return Err(DumperError::WaitPidError(child, e));
                }
            }
        }
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            // On x86, the stack pointer is NULL or -1, when executing trusted code in
            // the seccomp sandbox. Not only does this cause difficulties down the line
            // when trying to dump the thread's stack, it also results in the minidumps
            // containing information about the trusted threads. This information is
            // generally completely meaningless and just pollutes the minidumps.
            // We thus test the stack pointer and exclude any threads that are part of
            // the seccomp sandbox's trusted code.
            let skip_thread;
            let regs = thread_info::ThreadInfo::getregs(pid.into());
            if let Ok(regs) = regs {
                #[cfg(target_arch = "x86_64")]
                {
                    skip_thread = regs.rsp == 0;
                }
                #[cfg(target_arch = "x86")]
                {
                    skip_thread = regs.esp == 0;
                }
            } else {
                skip_thread = true;
            }
            if skip_thread {
                ptrace_detach(child)?;
                return Err(DumperError::DetachSkippedThread(child));
            }
        }
        Ok(())
    }

    /// Resumes a thread by detaching from it.
    pub fn resume_thread(child: Pid) -> Result<(), DumperError> {
        ptrace_detach(child)
    }

    pub fn suspend_threads(&mut self, mut soft_errors: impl WriteErrorList<DumperError>) {
        // Iterate over all threads and try to suspend them.
        // If the thread either disappeared before we could attach to it, or if
        // it was part of the seccomp sandbox's trusted code, it is OK to
        // silently drop it from the minidump.
        self.threads.retain(|x| match Self::suspend_thread(x.tid) {
            Ok(()) => true,
            Err(e) => {
                soft_errors.push(e);
                false
            }
        });

        self.threads_suspended = true;

        failspot::failspot!(<crate::FailSpotName>::SuspendThreads soft_errors.push(DumperError::PtraceAttachError(1234, nix::Error::EPERM)))
    }

    pub fn resume_threads(&mut self, mut soft_errors: impl WriteErrorList<DumperError>) {
        if self.threads_suspended {
            for thread in &self.threads {
                match Self::resume_thread(thread.tid) {
                    Ok(()) => (),
                    Err(e) => {
                        soft_errors.push(e);
                    }
                }
            }
        }
        self.threads_suspended = false;
    }

    /// Send SIGSTOP to the process so that we can get a consistent state.
    ///
    /// This will block waiting for the process to stop until `timeout` has passed.
    fn stop_process(&mut self, timeout: Duration) -> Result<(), StopProcessError> {
        failspot!(StopProcess bail(nix::Error::EPERM));

        signal::kill(nix::unistd::Pid::from_raw(self.pid), Some(signal::SIGSTOP))?;

        // Something like waitpid for non-child processes would be better, but we have no such
        // tool, so we poll the status.
        const POLL_INTERVAL: Duration = Duration::from_millis(1);
        let proc_file = format!("/proc/{}/stat", self.pid);
        let end = Instant::now() + timeout;

        loop {
            if let Ok(ProcState::Stopped) = Stat::from_file(&proc_file)?.state() {
                return Ok(());
            }

            std::thread::sleep(POLL_INTERVAL);
            if Instant::now() > end {
                return Err(StopProcessError::Timeout);
            }
        }
    }

    /// Send SIGCONT to the process to continue.
    ///
    /// Unlike `stop_process`, this function does not wait for the process to continue.
    fn continue_process(&mut self) -> Result<(), ContinueProcessError> {
        signal::kill(nix::unistd::Pid::from_raw(self.pid), Some(signal::SIGCONT))?;
        Ok(())
    }

    /// Parse /proc/$pid/task to list all the threads of the process identified by
    /// pid.
    fn enumerate_threads(
        &mut self,
        mut soft_errors: impl WriteErrorList<InitError>,
    ) -> Result<(), InitError> {
        let pid = self.pid;
        let filename = format!("/proc/{pid}/task");
        let task_path = path::PathBuf::from(&filename);
        if !task_path.is_dir() {
            return Err(InitError::ProcPidTaskNotDirectory(filename));
        }

        for entry in std::fs::read_dir(task_path).map_err(|e| InitError::IOError(filename, e))? {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    soft_errors.push(InitError::ReadProcessThreadEntryFailed(e));
                    continue;
                }
            };
            let file_name = entry.file_name();
            let tid = match file_name.to_str().and_then(|name| name.parse::<Pid>().ok()) {
                Some(tid) => tid,
                None => {
                    soft_errors.push(InitError::ProcessTaskEntryNotTid(file_name));
                    continue;
                }
            };

            // Read the thread-name (if there is any)
            let name_result = failspot!(if ThreadName {
                Err(std::io::Error::other(
                    "testing requested failure reading thread name",
                ))
            } else {
                std::fs::read_to_string(format!("/proc/{pid}/task/{tid}/comm"))
            });

            let name = match name_result {
                Ok(name) => Some(name.trim_end().to_string()),
                Err(e) => {
                    soft_errors.push(InitError::ReadThreadNameFailed(e));
                    None
                }
            };

            self.threads.push(Thread { tid, name });
        }

        Ok(())
    }

    fn enumerate_mappings(&mut self) -> Result<(), InitError> {
        // linux_gate_loc is the beginning of the kernel's mapping of
        // linux-gate.so in the process.  It doesn't actually show up in the
        // maps list as a filename, but it can be found using the AT_SYSINFO_EHDR
        // aux vector entry, which gives the information necessary to special
        // case its entry when creating the list of mappings.
        // See http://www.trilithium.com/johan/2005/08/linux-gate/ for more
        // information.
        let maps_path = format!("/proc/{}/maps", self.pid);
        let maps_file =
            std::fs::File::open(&maps_path).map_err(|e| InitError::IOError(maps_path, e))?;

        let maps = procfs_core::process::MemoryMaps::from_read(maps_file)
            .map_err(InitError::ReadProcessMapFileFailed)?;

        self.mappings = MappingInfo::aggregate(maps, self.auxv.get_linux_gate_address())
            .map_err(InitError::AggregateMappingsFailed)?;

        // Although the initial executable is usually the first mapping, it's not
        // guaranteed (see http://crosbug.com/25355); therefore, try to use the
        // actual entry point to find the mapping.
        if let Some(entry_point_loc) = self
            .auxv
            .get_entry_address()
            .map(|u| usize::try_from(u).unwrap())
        {
            // If this module contains the entry-point, and it's not already the first
            // one, then we need to make it be first.  This is because the minidump
            // format assumes the first module is the one that corresponds to the main
            // executable (as codified in
            // processor/minidump.cc:MinidumpModuleList::GetMainModule()).
            if let Some(entry_mapping_idx) = self.mappings.iter().position(|mapping| {
                (mapping.start_address..mapping.start_address + mapping.size)
                    .contains(&entry_point_loc)
            }) {
                self.mappings.swap(0, entry_mapping_idx);
            }
        }
        Ok(())
    }

    /// Read thread info from /proc/$pid/status.
    /// Fill out the |tgid|, |ppid| and |pid| members of |info|. If unavailable,
    /// these members are set to -1. Returns true if all three members are
    /// available.
    pub fn get_thread_info_by_index(&self, index: usize) -> Result<ThreadInfo, ThreadInfoError> {
        if index > self.threads.len() {
            return Err(ThreadInfoError::IndexOutOfBounds(index, self.threads.len()));
        }

        ThreadInfo::create(self.pid, self.threads[index].tid)
    }

    // Returns a valid stack pointer and the mapping that contains the stack.
    // The stack pointer will usually point within this mapping, but it might
    // not in case of stack overflows, hence the returned pointer might be
    // different from the one that was passed in.
    pub fn get_stack_info(&self, int_stack_pointer: usize) -> Result<(usize, usize), DumperError> {
        // Round the stack pointer to the nearest page, this will cause us to
        // capture data below the stack pointer which might still be relevant.
        let mut stack_pointer = int_stack_pointer & !(self.page_size - 1);
        let mut mapping = self.find_mapping(stack_pointer);

        // The guard page has been 1 MiB in size since kernel 4.12, older
        // kernels used a 4 KiB one instead. Note the saturating add, as 32-bit
        // processes can have a stack pointer within 1MiB of usize::MAX
        let guard_page_max_addr = stack_pointer.saturating_add(1024 * 1024);

        // If we found no mapping, or the mapping we found has no permissions
        // then we might have hit a guard page, try looking for a mapping in
        // addresses past the stack pointer. Stack grows towards lower addresses
        // on the platforms we care about so the stack should appear after the
        // guard page.
        while !Self::may_be_stack(mapping) && (stack_pointer <= guard_page_max_addr) {
            stack_pointer += self.page_size;
            mapping = self.find_mapping(stack_pointer);
        }

        mapping
            .map(|mapping| {
                let valid_stack_pointer = if mapping.contains_address(stack_pointer) {
                    stack_pointer
                } else {
                    mapping.start_address
                };

                let stack_len = mapping.size - (valid_stack_pointer - mapping.start_address);
                (valid_stack_pointer, stack_len)
            })
            .ok_or(DumperError::NoStackPointerMapping)
    }

    fn may_be_stack(mapping: Option<&MappingInfo>) -> bool {
        if let Some(mapping) = mapping {
            return mapping
                .permissions
                .intersects(MMPermissions::READ | MMPermissions::WRITE);
        }

        false
    }

    pub fn sanitize_stack_copy(
        &self,
        stack_copy: &mut [u8],
        stack_pointer: usize,
        sp_offset: usize,
    ) -> Result<(), DumperError> {
        // We optimize the search for containing mappings in three ways:
        // 1) We expect that pointers into the stack mapping will be common, so
        //    we cache that address range.
        // 2) The last referenced mapping is a reasonable predictor for the next
        //    referenced mapping, so we test that first.
        // 3) We precompute a bitfield based upon bits 32:32-n of the start and
        //    stop addresses, and use that to short circuit any values that can
        //    not be pointers. (n=11)
        let defaced;
        #[cfg(target_pointer_width = "64")]
        {
            defaced = 0x0defaced0defacedusize.to_ne_bytes();
        }
        #[cfg(target_pointer_width = "32")]
        {
            defaced = 0x0defacedusize.to_ne_bytes();
        };
        // the bitfield length is 2^test_bits long.
        let test_bits = 11;
        // byte length of the corresponding array.
        let array_size: usize = 1 << (test_bits - 3);
        let array_mask = array_size - 1;
        // The amount to right shift pointers by. This captures the top bits
        // on 32 bit architectures. On 64 bit architectures this would be
        // uninformative so we take the same range of bits.
        let shift = 32 - 11;
        // let MappingInfo* last_hit_mapping = nullptr;
        // let MappingInfo* hit_mapping = nullptr;
        let stack_mapping = self.find_mapping_no_bias(stack_pointer);
        let mut last_hit_mapping: Option<&MappingInfo> = None;
        // The magnitude below which integers are considered to be to be
        // 'small', and not constitute a PII risk. These are included to
        // avoid eliding useful register values.
        let small_int_magnitude: isize = 4096;

        let mut could_hit_mapping = vec![0; array_size];
        // Initialize the bitfield such that if the (pointer >> shift)'th
        // bit, modulo the bitfield size, is not set then there does not
        // exist a mapping in mappings that would contain that pointer.
        for mapping in &self.mappings {
            if !mapping.is_executable() {
                continue;
            }
            // For each mapping, work out the (unmodulo'ed) range of bits to
            // set.
            let mut start = mapping.start_address;
            let mut end = start + mapping.size;
            start >>= shift;
            end >>= shift;
            for bit in start..=end {
                // Set each bit in the range, applying the modulus.
                could_hit_mapping[(bit >> 3) & array_mask] |= 1 << (bit & 7);
            }
        }

        // Zero memory that is below the current stack pointer.
        let offset =
            (sp_offset + std::mem::size_of::<usize>() - 1) & !(std::mem::size_of::<usize>() - 1);
        for x in &mut stack_copy[0..offset] {
            *x = 0;
        }
        let mut chunks = stack_copy[offset..].chunks_exact_mut(std::mem::size_of::<usize>());

        // Apply sanitization to each complete pointer-aligned word in the
        // stack.
        for sp in &mut chunks {
            let addr = usize::from_ne_bytes(sp.to_vec().as_slice().try_into()?);
            let addr_signed = isize::from_ne_bytes(sp.to_vec().as_slice().try_into()?);

            if addr <= small_int_magnitude as usize && addr_signed >= -small_int_magnitude {
                continue;
            }

            if let Some(stack_map) = stack_mapping {
                if stack_map.contains_address(addr) {
                    continue;
                }
            }
            if let Some(last_hit) = last_hit_mapping {
                if last_hit.contains_address(addr) {
                    continue;
                }
            }

            let test = addr >> shift;
            if could_hit_mapping[(test >> 3) & array_mask] & (1 << (test & 7)) != 0 {
                if let Some(hit_mapping) = self.find_mapping_no_bias(addr) {
                    if hit_mapping.is_executable() {
                        last_hit_mapping = Some(hit_mapping);
                        continue;
                    }
                }
            }
            sp.copy_from_slice(&defaced);
        }
        // Zero any partial word at the top of the stack, if alignment is
        // such that that is required.
        for sp in chunks.into_remainder() {
            *sp = 0;
        }
        Ok(())
    }

    // Find the mapping which the given memory address falls in.
    pub fn find_mapping(&self, address: usize) -> Option<&MappingInfo> {
        self.mappings
            .iter()
            .find(|map| address >= map.start_address && address - map.start_address < map.size)
    }

    // Find the mapping which the given memory address falls in. Uses the
    // unadjusted mapping address range from the kernel, rather than the
    // biased range.
    pub fn find_mapping_no_bias(&self, address: usize) -> Option<&MappingInfo> {
        self.mappings.iter().find(|map| {
            address >= map.system_mapping_info.start_address
                && address < map.system_mapping_info.end_address
        })
    }

    pub fn from_process_memory_for_index<T: module_reader::ReadFromModule>(
        &mut self,
        idx: usize,
    ) -> Result<T, DumperError> {
        assert!(idx < self.mappings.len());

        Self::from_process_memory_for_mapping(&self.mappings[idx], self.pid)
    }

    pub fn from_process_memory_for_mapping<T: module_reader::ReadFromModule>(
        mapping: &MappingInfo,
        pid: Pid,
    ) -> Result<T, DumperError> {
        Ok(T::read_from_module(
            module_reader::ProcessReader::new(pid, mapping.start_address).into(),
        )?)
    }
}
