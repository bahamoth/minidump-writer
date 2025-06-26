use super::errors::{Error, ProcessError};
use mach2::mach_types as mt;
use std::sync::atomic::{AtomicBool, Ordering};

static DUMPER_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// iOS-specific task dumper that only supports self-process dumping
/// due to iOS security restrictions
pub struct TaskDumper {
    #[allow(dead_code)]
    task: mt::task_t,
    #[allow(dead_code)]
    page_size: i64,
    #[allow(dead_code)]
    is_self_process: bool,
}

#[allow(dead_code)]
impl TaskDumper {
    /// Constructs a TaskDumper for the current process only (iOS restriction)
    pub fn new_self_process() -> Result<Self, Error> {
        // Ensure we only create one instance for signal safety
        if DUMPER_INITIALIZED.swap(true, Ordering::SeqCst) {
            return Err(Error::SecurityRestriction(
                "TaskDumper already initialized".to_string(),
            ));
        }

        // SAFETY: mach_task_self() is a macro that returns the current task port
        let task = unsafe { mach2::traps::mach_task_self() };
        
        Ok(Self {
            task,
            // SAFETY: syscall
            page_size: unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as i64,
            is_self_process: true,
        })
    }

    /// Reads a block of memory from the current task
    ///
    /// # Safety
    /// This function is signal-safe when reading from pre-validated memory regions
    pub fn read_task_memory<T>(&self, address: u64, count: usize) -> Result<Vec<T>, Error>
    where
        T: Sized + Clone,
    {
        if !self.is_self_process {
            return Err(Error::IOSProcessError(ProcessError::CrossProcessNotSupported));
        }

        // Validate memory before reading on iOS
        let length = (count * std::mem::size_of::<T>()) as u64;
        if !self.validate_memory_range(address as usize, length as usize) {
            return Err(Error::MemoryValidation {
                addr: address as usize,
                size: length as usize,
            });
        }

        // Since we're reading from our own process, we can directly access memory
        // This is signal-safe as it doesn't allocate
        let mut buffer = Vec::with_capacity(count);
        
        // SAFETY: We've validated the memory range above
        unsafe {
            let src = address as *const T;
            let slice = std::slice::from_raw_parts(src, count);
            buffer.extend_from_slice(slice);
        }

        Ok(buffer)
    }

    /// Validates that a memory range is readable
    fn validate_memory_range(&self, addr: usize, size: usize) -> bool {
        use mach2::vm::*;
        use mach2::vm_region::*;
        use mach2::kern_return::KERN_SUCCESS;
        use mach2::message::mach_msg_type_number_t;

        unsafe {
            let mut info: vm_region_basic_info_64 = std::mem::zeroed();
            let mut info_size = std::mem::size_of_val(&info) as mach_msg_type_number_t;
            let mut object_name: mach2::port::mach_port_t = 0;
            let mut region_addr = addr as mach2::vm_types::mach_vm_address_t;
            let mut region_size = size as mach2::vm_types::mach_vm_size_t;
            
            let kr = mach_vm_region(
                self.task,
                &mut region_addr,
                &mut region_size,
                VM_REGION_BASIC_INFO_64,
                &mut info as *mut _ as vm_region_info_t,
                &mut info_size,
                &mut object_name,
            );
            
            kr == KERN_SUCCESS && (info.protection & mach2::vm_prot::VM_PROT_READ) != 0
        }
    }

    /// Reads a null-terminated string from memory
    pub fn read_string(&self, addr: u64, max_len: Option<usize>) -> Result<Option<String>, Error> {
        let max_len = max_len.unwrap_or(4096);
        
        // For iOS, we read byte by byte until null terminator
        // This is less efficient but safer for signal handling
        let mut bytes = Vec::new();
        
        for i in 0..max_len {
            match self.read_task_memory::<u8>(addr + i as u64, 1) {
                Ok(byte_vec) => {
                    if byte_vec[0] == 0 {
                        break;
                    }
                    bytes.push(byte_vec[0]);
                }
                Err(_) => break,
            }
        }

        if bytes.is_empty() {
            Ok(None)
        } else {
            String::from_utf8(bytes).map(Some).map_err(|_e| Error::ProcParseError)
        }
    }

    /// Retrieves thread state for a thread in the current process
    pub fn read_thread_state(&self, tid: u32) -> Result<ThreadState, Error> {
        use mach2::thread_act::*;

        let mut state = ThreadState::default();
        
        // ARM64 thread state constant for iOS
        const ARM_THREAD_STATE64: i32 = 6;
        
        unsafe {
            let kr = thread_get_state(
                tid,
                ARM_THREAD_STATE64,
                state.state.as_mut_ptr() as *mut _,
                &mut state.state_size,
            );
            
            if kr != mach2::kern_return::KERN_SUCCESS {
                return Err(Error::MachError(kr));
            }
        }

        Ok(state)
    }

    /// Get the task port
    pub fn task(&self) -> mt::task_t {
        self.task
    }
}

/// Thread state for iOS (arm64)
#[repr(C)]
pub struct ThreadState {
    pub state: [u32; 68], // ARM_THREAD_STATE64_COUNT
    pub state_size: u32,
}

impl Default for ThreadState {
    fn default() -> Self {
        Self {
            state: [0; 68],
            state_size: 68,
        }
    }
}

impl Drop for TaskDumper {
    fn drop(&mut self) {
        DUMPER_INITIALIZED.store(false, Ordering::SeqCst);
    }
}