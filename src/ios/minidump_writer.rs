use crate::{
    dir_section::{DirSection, DumpBuf},
    mem_writer::*,
    minidump_format::{self, MDMemoryDescriptor, MDRawDirectory, MDRawHeader},
};
use super::{
    crash_handler::{SignalSafeWriter, get_crash_context, IOSCrashContext},
    errors::Error,
    system_info::{SignalSafeSystemInfo},
    task_dumper::TaskDumper,
};
use std::io::{Seek, Write};

pub use mach2::mach_types::{task_t, thread_t};

type Result<T> = std::result::Result<T, Error>;
type WriterList = Vec<Box<dyn FnMut(&mut MinidumpWriter, &mut DumpBuf, &TaskDumper) -> Result<MDRawDirectory>>>;

/// iOS-specific MinidumpWriter that only supports self-process dumping
pub struct MinidumpWriter {
    /// The crash context as captured by signal handler
    pub(crate) crash_context: Option<IOSCrashContext>,
    /// List of raw blocks of memory we've written into the stream
    #[allow(dead_code)]
    pub(crate) memory_blocks: Vec<MDMemoryDescriptor>,
    /// The task being dumped (always self on iOS)
    #[allow(dead_code)]
    pub(crate) task: task_t,
    /// The handler thread
    #[allow(dead_code)]
    pub(crate) handler_thread: thread_t,
    /// Pre-initialized system info for signal safety
    #[allow(dead_code)]
    pub(crate) system_info: Option<SignalSafeSystemInfo>,
}

impl MinidumpWriter {
    /// Creates a minidump writer for the current process (iOS limitation)
    pub fn new() -> Result<Self> {
        // SAFETY: These are macros that return current task/thread
        let task = unsafe { mach2::traps::mach_task_self() };
        let handler_thread = unsafe { mach2::mach_init::mach_thread_self() };
        
        // Pre-initialize system info for signal safety
        let system_info = SignalSafeSystemInfo::new().ok();
        
        Ok(Self {
            crash_context: None,
            memory_blocks: Vec::new(),
            task,
            handler_thread,
            system_info,
        })
    }

    /// Creates a minidump writer with crash context from signal handler
    pub fn with_crash_context(crash_context: IOSCrashContext) -> Result<Self> {
        let mut writer = Self::new()?;
        writer.crash_context = Some(crash_context);
        Ok(writer)
    }

    /// Creates a minidump writer from current signal handler context
    pub fn from_signal_handler() -> Result<Self> {
        let crash_context = get_crash_context()
            .ok_or(Error::NoCrashContext)?;
        Self::with_crash_context(crash_context)
    }

    /// Writes a minidump for normal operation (non-signal context)
    pub fn dump(&mut self, destination: &mut (impl Write + Seek)) -> Result<Vec<u8>> {
        let dumper = TaskDumper::new_self_process()?;
        
        let writers = self.create_writers();
        
        let mut buffer = DumpBuf::with_capacity(1024 * 1024); // Reserve 1MB

        self.write_minidump(&mut buffer, &dumper, writers)?;
        
        destination.write_all(&buffer)?;
        Ok(buffer.into())
    }

    /// Signal-safe minidump writing for crash handlers
    /// This uses pre-allocated buffers and avoids allocations
    ///
    /// # Safety
    ///
    /// This function must only be called from a signal handler context.
    /// It assumes exclusive access to global crash buffers.
    pub unsafe fn dump_signal_safe(&mut self) -> Result<()> {
        let mut writer = SignalSafeWriter::new();
        
        // Write a simplified minidump header
        let header = MDRawHeader {
            signature: minidump_format::MD_HEADER_SIGNATURE,
            version: minidump_format::MD_HEADER_VERSION,
            stream_count: 0, // Will be updated
            stream_directory_rva: std::mem::size_of::<MDRawHeader>() as u32,
            checksum: 0,
            time_date_stamp: 0, // Would require allocation to get current time
            flags: 0,
        };
        
        let header_bytes = std::slice::from_raw_parts(
            &header as *const _ as *const u8,
            std::mem::size_of::<MDRawHeader>(),
        );
        
        if !writer.write(header_bytes) {
            return Err(Error::FileWrite(std::io::Error::new(
                std::io::ErrorKind::OutOfMemory,
                "Buffer full",
            )));
        }
        
        // Write basic crash info
        if let Some(ref ctx) = self.crash_context {
            // Signal-safe string formatting without allocation
            let sig_bytes = b"Signal: ";
            writer.write(sig_bytes);
            
            // Write signal number as ASCII digits
            let signo = ctx.siginfo.si_signo;
            let mut num_buf = [0u8; 10];
            let mut n = signo as u32;
            let mut i = 9;
            loop {
                num_buf[i] = b'0' + (n % 10) as u8;
                n /= 10;
                if n == 0 {
                    break;
                }
                i -= 1;
            }
            writer.write(&num_buf[i..]);
            writer.write(b"\n");
        }
        
        // Flush to file descriptor
        writer.flush_to_fd();
        
        Ok(())
    }

    /// Creates the list of writers for minidump sections
    fn create_writers(&self) -> WriterList {
        let mut writers: WriterList = vec![
            Box::new(|mw, buffer, dumper| mw.write_thread_list(buffer, dumper)),
            Box::new(|mw, buffer, dumper| mw.write_memory_list(buffer, dumper)),
            Box::new(|mw, buffer, dumper| mw.write_system_info(buffer, dumper)),
            Box::new(|mw, buffer, dumper| mw.write_module_list(buffer, dumper)),
            Box::new(|mw, buffer, dumper| mw.write_misc_info(buffer, dumper)),
            Box::new(|mw, buffer, dumper| mw.write_breakpad_info(buffer, dumper)),
            Box::new(|mw, buffer, dumper| mw.write_thread_names(buffer, dumper)),
        ];

        // Exception stream if we have crash context
        if self.crash_context.is_some() {
            writers.push(Box::new(|mw, buffer, dumper| {
                mw.write_exception(buffer, dumper)
            }));
        }

        writers
    }

    /// Internal minidump writing logic
    fn write_minidump(
        &mut self,
        buffer: &mut DumpBuf,
        dumper: &TaskDumper,
        writers: WriterList,
    ) -> Result<()> {
        // Write header placeholder
        let mut header = MDRawHeader {
            signature: minidump_format::MD_HEADER_SIGNATURE,
            version: minidump_format::MD_HEADER_VERSION,
            stream_count: writers.len() as u32,
            stream_directory_rva: 0,
            checksum: 0,
            time_date_stamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as u32,
            flags: 0,
        };

        let mut header_section = MemoryWriter::<MDRawHeader>::alloc(buffer)?;

        // Create a temporary in-memory destination for directory section
        let mut temp_dest = std::io::Cursor::new(Vec::new());
        let mut directory_section = DirSection::new(buffer, writers.len() as u32, &mut temp_dest)?;
        header.stream_directory_rva = directory_section.position();

        for mut writer in writers.into_iter() {
            let dir_entry = writer(self, buffer, dumper)?;
            directory_section.write_to_file(buffer, Some(dir_entry))?;
        }

        header_section.set_value(buffer, header)?;
        
        Ok(())
    }

    // Stream writer methods will be implemented in separate files
    fn write_thread_list(&mut self, _buffer: &mut DumpBuf, _dumper: &TaskDumper) -> Result<MDRawDirectory> {
        // TODO: Implement thread list writing
        Ok(MDRawDirectory {
            stream_type: minidump_format::MDStreamType::ThreadListStream as u32,
            location: MDMemoryDescriptor {
                start_of_memory_range: 0,
                memory: minidump_format::MDLocationDescriptor {
                    data_size: 0,
                    rva: 0,
                },
            }.memory,
        })
    }

    fn write_memory_list(&mut self, _buffer: &mut DumpBuf, _dumper: &TaskDumper) -> Result<MDRawDirectory> {
        // TODO: Implement memory list writing
        Ok(MDRawDirectory {
            stream_type: minidump_format::MDStreamType::MemoryListStream as u32,
            location: MDMemoryDescriptor {
                start_of_memory_range: 0,
                memory: minidump_format::MDLocationDescriptor {
                    data_size: 0,
                    rva: 0,
                },
            }.memory,
        })
    }

    fn write_system_info(&mut self, _buffer: &mut DumpBuf, _dumper: &TaskDumper) -> Result<MDRawDirectory> {
        // TODO: Implement system info writing
        Ok(MDRawDirectory {
            stream_type: minidump_format::MDStreamType::SystemInfoStream as u32,
            location: MDMemoryDescriptor {
                start_of_memory_range: 0,
                memory: minidump_format::MDLocationDescriptor {
                    data_size: 0,
                    rva: 0,
                },
            }.memory,
        })
    }

    fn write_module_list(&mut self, _buffer: &mut DumpBuf, _dumper: &TaskDumper) -> Result<MDRawDirectory> {
        // TODO: Implement module list writing
        Ok(MDRawDirectory {
            stream_type: minidump_format::MDStreamType::ModuleListStream as u32,
            location: MDMemoryDescriptor {
                start_of_memory_range: 0,
                memory: minidump_format::MDLocationDescriptor {
                    data_size: 0,
                    rva: 0,
                },
            }.memory,
        })
    }

    fn write_misc_info(&mut self, _buffer: &mut DumpBuf, _dumper: &TaskDumper) -> Result<MDRawDirectory> {
        // TODO: Implement misc info writing
        Ok(MDRawDirectory {
            stream_type: minidump_format::MDStreamType::MiscInfoStream as u32,
            location: MDMemoryDescriptor {
                start_of_memory_range: 0,
                memory: minidump_format::MDLocationDescriptor {
                    data_size: 0,
                    rva: 0,
                },
            }.memory,
        })
    }

    fn write_breakpad_info(&mut self, _buffer: &mut DumpBuf, _dumper: &TaskDumper) -> Result<MDRawDirectory> {
        // TODO: Implement breakpad info writing
        Ok(MDRawDirectory {
            stream_type: minidump_format::MDStreamType::BreakpadInfoStream as u32,
            location: MDMemoryDescriptor {
                start_of_memory_range: 0,
                memory: minidump_format::MDLocationDescriptor {
                    data_size: 0,
                    rva: 0,
                },
            }.memory,
        })
    }

    fn write_thread_names(&mut self, _buffer: &mut DumpBuf, _dumper: &TaskDumper) -> Result<MDRawDirectory> {
        // TODO: Implement thread names writing
        Ok(MDRawDirectory {
            stream_type: minidump_format::MDStreamType::ThreadNamesStream as u32,
            location: MDMemoryDescriptor {
                start_of_memory_range: 0,
                memory: minidump_format::MDLocationDescriptor {
                    data_size: 0,
                    rva: 0,
                },
            }.memory,
        })
    }

    fn write_exception(&mut self, _buffer: &mut DumpBuf, _dumper: &TaskDumper) -> Result<MDRawDirectory> {
        // TODO: Implement exception writing
        Ok(MDRawDirectory {
            stream_type: minidump_format::MDStreamType::ExceptionStream as u32,
            location: MDMemoryDescriptor {
                start_of_memory_range: 0,
                memory: minidump_format::MDLocationDescriptor {
                    data_size: 0,
                    rva: 0,
                },
            }.memory,
        })
    }
}