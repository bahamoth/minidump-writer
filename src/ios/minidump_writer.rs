use crate::{
    dir_section::{DirSection, DumpBuf},
    mem_writer::*,
    minidump_format::{self, MDMemoryDescriptor, MDRawDirectory, MDRawHeader},
    ios::task_dumper::TaskDumper,
};
use std::io::{Seek, Write};

pub use mach2::mach_types::{task_t, thread_t};

type Result<T> = std::result::Result<T, super::errors::WriterError>;

pub struct MinidumpWriter {
    /// The crash context as captured by an exception handler
    pub(crate) crash_context: Option<crash_context::CrashContext>,
    /// List of raw blocks of memory we've written into the stream. These are
    /// referenced by other streams (eg thread list)
    pub(crate) memory_blocks: Vec<MDMemoryDescriptor>,
    /// The task being dumped (iOS only supports self-process)
    pub(crate) task: task_t,
    /// The handler thread, so it can be ignored/deprioritized
    pub(crate) handler_thread: thread_t,
}

impl MinidumpWriter {
    /// Creates a minidump writer for the current process.
    /// iOS only supports self-process dumping due to sandboxing restrictions.
    ///
    /// ```
    /// use minidump_writer::ios::MinidumpWriter;
    ///
    /// // Creates a writer for the current process and thread
    /// let mdw = MinidumpWriter::new();
    /// ```
    pub fn new() -> Self {
        Self {
            crash_context: None,
            memory_blocks: Vec::new(),
            // SAFETY: syscall to get current task
            task: unsafe { mach2::traps::mach_task_self() },
            // SAFETY: syscall to get current thread
            handler_thread: unsafe { mach2::mach_init::mach_thread_self() },
        }
    }

    /// Creates a minidump writer with the specified crash context
    /// for the current process
    pub fn with_crash_context(crash_context: crash_context::CrashContext) -> Self {
        // On iOS, we can only dump the current process
        debug_assert_eq!(crash_context.task, unsafe { mach2::traps::mach_task_self() });
        
        let handler_thread = crash_context.handler_thread;

        Self {
            crash_context: Some(crash_context),
            memory_blocks: Vec::new(),
            task: unsafe { mach2::traps::mach_task_self() },
            handler_thread,
        }
    }

    /// Writes a minidump to the specified destination, returning the raw minidump
    /// contents upon success
    pub fn dump(&mut self, destination: &mut (impl Write + Seek)) -> Result<Vec<u8>> {
        let writers = {
            #[allow(clippy::type_complexity)]
            let mut writers: Vec<
                Box<dyn FnMut(&mut Self, &mut DumpBuf, &TaskDumper) -> Result<MDRawDirectory>>,
            > = vec![
                Box::new(|mw, buffer, dumper| mw.write_thread_list(buffer, dumper)),
                Box::new(|mw, buffer, dumper| mw.write_memory_list(buffer, dumper)),
                Box::new(|mw, buffer, dumper| mw.write_system_info(buffer, dumper)),
                Box::new(|mw, buffer, dumper| mw.write_module_list(buffer, dumper)),
                Box::new(|mw, buffer, dumper| mw.write_misc_info(buffer, dumper)),
                Box::new(|mw, buffer, dumper| mw.write_breakpad_info(buffer, dumper)),
                Box::new(|mw, buffer, dumper| mw.write_thread_names(buffer, dumper)),
            ];

            // Exception stream needs to be the last entry in this array as it may
            // be omitted in the case where the minidump is written without an
            // exception.
            if self
                .crash_context
                .as_ref()
                .and_then(|cc| cc.exception.as_ref())
                .is_some()
            {
                writers.push(Box::new(|mw, buffer, dumper| {
                    mw.write_exception(buffer, dumper)
                }));
            }

            writers
        };

        let num_writers = writers.len() as u32;
        let mut buffer = Buffer::with_capacity(0);

        let mut header_section = MemoryWriter::<MDRawHeader>::alloc(&mut buffer)?;
        let mut dir_section = DirSection::new(&mut buffer, num_writers, destination)?;

        let header = MDRawHeader {
            signature: minidump_format::MD_HEADER_SIGNATURE,
            version: minidump_format::MD_HEADER_VERSION,
            stream_count: num_writers,
            stream_directory_rva: dir_section.position(),
            checksum: 0, /* Can be 0.  In fact, that's all that's
                          * been found in minidump files. */
            time_date_stamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as u32, // TODO: This is not Y2038 safe, but thats how its currently defined as
            flags: 0,
        };
        header_section.set_value(&mut buffer, header)?;

        // Ensure the header gets flushed. If we crash somewhere below,
        // we should have a mostly-intact dump
        dir_section.write_to_file(&mut buffer, None)?;

        let dumper = super::task_dumper::TaskDumper::new(self.task);

        for mut writer in writers {
            let dirent = writer(self, &mut buffer, &dumper)?;
            dir_section.write_to_file(&mut buffer, Some(dirent))?;
        }

        Ok(buffer.into())
    }

    /// Retrieves the list of active threads in the target process, except
    /// the handler thread if it is known, to simplify dump analysis
    #[inline]
    pub(crate) fn threads(&self, dumper: &TaskDumper) -> ActiveThreads {
        ActiveThreads {
            threads: dumper.read_threads().unwrap_or_default(),
            handler_thread: self.handler_thread,
            i: 0,
        }
    }
}

impl Default for MinidumpWriter {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) struct ActiveThreads {
    threads: &'static [u32],
    handler_thread: u32,
    i: usize,
}

impl ActiveThreads {
    #[inline]
    pub(crate) fn len(&self) -> usize {
        let mut len = self.threads.len();

        if self.handler_thread != mach2::port::MACH_PORT_NULL {
            len -= 1;
        }

        len
    }
}

impl Iterator for ActiveThreads {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        while self.i < self.threads.len() {
            let i = self.i;
            self.i += 1;

            if self.threads[i] != self.handler_thread {
                return Some(self.threads[i]);
            }
        }

        None
    }
}

// TODO: Add write methods for each stream type
impl MinidumpWriter {
    fn write_thread_list(&mut self, buffer: &mut DumpBuf, dumper: &TaskDumper) -> Result<MDRawDirectory> {
        todo!("Implement thread list writing")
    }

    fn write_memory_list(&mut self, buffer: &mut DumpBuf, dumper: &TaskDumper) -> Result<MDRawDirectory> {
        todo!("Implement memory list writing")
    }

    fn write_system_info(&mut self, buffer: &mut DumpBuf, dumper: &TaskDumper) -> Result<MDRawDirectory> {
        todo!("Implement system info writing")
    }

    fn write_module_list(&mut self, buffer: &mut DumpBuf, dumper: &TaskDumper) -> Result<MDRawDirectory> {
        todo!("Implement module list writing")
    }

    fn write_misc_info(&mut self, buffer: &mut DumpBuf, dumper: &TaskDumper) -> Result<MDRawDirectory> {
        todo!("Implement misc info writing")
    }

    fn write_breakpad_info(&mut self, buffer: &mut DumpBuf, dumper: &TaskDumper) -> Result<MDRawDirectory> {
        todo!("Implement breakpad info writing")
    }

    fn write_thread_names(&mut self, buffer: &mut DumpBuf, dumper: &TaskDumper) -> Result<MDRawDirectory> {
        todo!("Implement thread names writing")
    }

    fn write_exception(&mut self, buffer: &mut DumpBuf, dumper: &TaskDumper) -> Result<MDRawDirectory> {
        todo!("Implement exception writing")
    }
}