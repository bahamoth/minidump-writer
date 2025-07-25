use crate::{
    apple::ios::{crash_context::IosCrashContext, task_dumper::TaskDumper},
    dir_section::{DirSection, DumpBuf},
    mem_writer::*,
    minidump_format::{self, MDMemoryDescriptor, MDRawDirectory, MDRawHeader},
};
use std::io::{Seek, Write};

pub use mach2::mach_types::{task_t, thread_t};

type Result<T> = std::result::Result<T, WriterError>;

#[derive(Debug, thiserror::Error)]
pub enum WriterError {
    #[error("Failed to write minidump header")]
    HeaderError,
    #[error("Failed to write directory: {0}")]
    DirectoryError(String),
    #[error("System info error: {0}")]
    SystemInfoError(#[from] super::streams::system_info::SystemInfoError),
    #[error("Stream error: {0}")]
    StreamError(#[from] super::streams::StreamError),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Memory writer error: {0}")]
    MemoryWriterError(String),
    #[error("Task dumper error: {0}")]
    TaskDumperError(String),
}

pub struct MinidumpWriter {
    /// The crash context as captured by an exception handler
    pub(crate) crash_context: Option<IosCrashContext>,
    /// List of raw blocks of memory we've written into the stream. These are
    /// referenced by other streams (eg thread list)
    pub(crate) memory_blocks: Vec<MDMemoryDescriptor>,
    /// The task being dumped (on iOS, always self)
    pub(crate) task: task_t,
    /// The handler thread, so it can be ignored/deprioritized
    pub(crate) handler_thread: Option<thread_t>,
}

impl MinidumpWriter {
    /// Creates a minidump writer for the current task (self-process only on iOS)
    pub fn new() -> Self {
        Self {
            crash_context: None,
            memory_blocks: Vec::new(),
            // SAFETY: syscalls
            task: unsafe { mach2::traps::mach_task_self() },
            handler_thread: None,
        }
    }

    /// Sets the crash context for the minidump.
    pub fn set_crash_context(&mut self, crash_context: IosCrashContext) {
        // On iOS, we can only dump the current process
        debug_assert_eq!(crash_context.task, unsafe {
            mach2::traps::mach_task_self()
        });

        self.handler_thread = Some(crash_context.handler_thread);
        self.crash_context = Some(crash_context);
    }

    /// Writes a minidump to the specified destination
    pub fn dump(&mut self, destination: &mut (impl Write + Seek)) -> Result<Vec<u8>> {
        let mut buffer = DumpBuf::new(0);
        let dumper =
            TaskDumper::new(self.task).map_err(|e| WriterError::TaskDumperError(e.to_string()))?;

        // Reserve space for header
        let header_size = std::mem::size_of::<MDRawHeader>();
        buffer
            .reserve(header_size)
            .map_err(|e| WriterError::MemoryWriterError(e.to_string()))?;

        let mut dir_section = DirSection::new(&mut buffer, 0, destination).map_err(|e| {
            WriterError::DirectoryError(format!("Failed to create directory section: {}", e))
        })?;

        // Write thread list stream first to get the context
        let (thread_list_dirent, crashing_thread_context) =
            crate::apple::ios::streams::thread_list::write(self, &mut buffer, &dumper)
                .map_err(WriterError::from)?;

        // Write exception stream
        let dirent = crate::apple::ios::streams::exception::write(
            self,
            &mut buffer,
            crashing_thread_context,
        )
        .map_err(WriterError::from)?;
        dir_section.write_entry(dirent).map_err(|e| {
            WriterError::DirectoryError(format!("Failed to write directory entry: {}", e))
        })?;

        dir_section.write_entry(thread_list_dirent).map_err(|e| {
            WriterError::DirectoryError(format!("Failed to write directory entry: {}", e))
        })?;

        // Write memory list stream
        let memory_list_dirent =
            crate::apple::ios::streams::memory_list::write(self, &mut buffer, &dumper)
                .map_err(WriterError::from)?;
        dir_section.write_entry(memory_list_dirent).map_err(|e| {
            WriterError::DirectoryError(format!("Failed to write directory entry: {}", e))
        })?;

        // TODO: Add other streams (module list, system info, etc.)

        // Write directory
        let directory_location = dir_section.position().map_err(|e| {
            WriterError::DirectoryError(format!("Failed to get directory position: {}", e))
        })?;
        dir_section
            .write_to_buffer(&mut buffer, None)
            .map_err(|e| {
                WriterError::DirectoryError(format!("Failed to write directory to buffer: {}", e))
            })?;

        // Write header
        let header = MDRawHeader {
            signature: minidump_format::MINIDUMP_SIGNATURE,
            version: minidump_format::MINIDUMP_VERSION,
            stream_count: dir_section.count(),
            stream_directory_rva: directory_location.rva,
            checksum: 0, // TODO: Calculate checksum
            time_date_stamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as u32,
            flags: 0,
        };

        // Write header at the beginning
        buffer
            .write_at(header, 0)
            .map_err(|e| WriterError::MemoryWriterError(e.to_string()))?;

        // Write to destination
        let bytes = buffer.as_bytes();
        destination.write_all(&bytes)?;

        Ok(bytes.to_vec())
    }
}

impl Default for MinidumpWriter {
    fn default() -> Self {
        Self::new()
    }
}
