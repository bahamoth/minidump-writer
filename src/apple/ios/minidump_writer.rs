use crate::{
    apple::ios::{crash_context::IosCrashContext, task_dumper::TaskDumper},
    dir_section::{DirSection, DumpBuf},
    mem_writer::*,
    minidump_format::{
        format::{MINIDUMP_SIGNATURE, MINIDUMP_VERSION},
        MDLocationDescriptor, MDMemoryDescriptor, MDRawDirectory, MDRawHeader,
    },
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
    /// Location of the crashing thread's context (used by exception stream)
    pub(crate) crashing_thread_context: Option<MDLocationDescriptor>,
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
            crashing_thread_context: None,
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
        let writers = {
            #[allow(clippy::type_complexity)]
            let mut writers: Vec<
                Box<dyn FnMut(&mut Self, &mut DumpBuf, &TaskDumper) -> Result<MDRawDirectory>>,
            > = vec![
                Box::new(|mw, buffer, dumper| mw.write_system_info(buffer, dumper)),
                Box::new(|mw, buffer, dumper| mw.write_thread_list(buffer, dumper)),
                Box::new(|mw, buffer, dumper| mw.write_memory_list(buffer, dumper)),
                Box::new(|mw, buffer, dumper| mw.write_module_list(buffer, dumper)),
            ];

            // Exception stream is added conditionally if we have crash context
            if self.crash_context.is_some() {
                writers.push(Box::new(|mw, buffer, dumper| {
                    mw.write_exception(buffer, dumper)
                }));
            }

            writers
        };

        let num_writers = writers.len() as u32;
        let mut buffer = DumpBuf::with_capacity(0);

        let mut header_section = MemoryWriter::<MDRawHeader>::alloc(&mut buffer)
            .map_err(|e| WriterError::MemoryWriterError(e.to_string()))?;

        let mut dir_section =
            DirSection::new(&mut buffer, num_writers, destination).map_err(|e| {
                WriterError::DirectoryError(format!("Failed to create directory section: {e}"))
            })?;

        let header = MDRawHeader {
            signature: MINIDUMP_SIGNATURE,
            version: MINIDUMP_VERSION,
            stream_count: num_writers,
            stream_directory_rva: dir_section.position(),
            checksum: 0,
            time_date_stamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as u32,
            flags: 0,
        };

        header_section
            .set_value(&mut buffer, header)
            .map_err(|e| WriterError::MemoryWriterError(e.to_string()))?;

        // Ensure the header gets flushed
        dir_section
            .write_to_file(&mut buffer, None)
            .map_err(|e| WriterError::DirectoryError(format!("Failed to flush header: {e}")))?;

        let dumper =
            TaskDumper::new(self.task).map_err(|e| WriterError::TaskDumperError(e.to_string()))?;

        for mut writer in writers {
            let dirent = writer(self, &mut buffer, &dumper)?;
            dir_section
                .write_to_file(&mut buffer, Some(dirent))
                .map_err(|e| {
                    WriterError::DirectoryError(format!("Failed to write directory entry: {e}"))
                })?;
        }

        Ok(buffer.into())
    }

    // Stream writer methods
    fn write_system_info(
        &mut self,
        buffer: &mut DumpBuf,
        _dumper: &TaskDumper,
    ) -> Result<MDRawDirectory> {
        crate::apple::ios::streams::system_info::write_system_info(buffer)
            .map_err(WriterError::from)
    }

    fn write_thread_list(
        &mut self,
        buffer: &mut DumpBuf,
        dumper: &TaskDumper,
    ) -> Result<MDRawDirectory> {
        let (dirent, context) =
            crate::apple::ios::streams::thread_list::write(self, buffer, dumper)
                .map_err(WriterError::from)?;

        // Store the crashing thread context for exception stream
        self.crashing_thread_context = context;

        Ok(dirent)
    }

    fn write_memory_list(
        &mut self,
        buffer: &mut DumpBuf,
        dumper: &TaskDumper,
    ) -> Result<MDRawDirectory> {
        crate::apple::ios::streams::memory_list::write(self, buffer, dumper)
            .map_err(WriterError::from)
    }

    fn write_module_list(
        &mut self,
        buffer: &mut DumpBuf,
        dumper: &TaskDumper,
    ) -> Result<MDRawDirectory> {
        crate::apple::ios::streams::module_list::write(self, buffer, dumper)
            .map_err(WriterError::from)
    }

    fn write_exception(
        &mut self,
        buffer: &mut DumpBuf,
        _dumper: &TaskDumper,
    ) -> Result<MDRawDirectory> {
        crate::apple::ios::streams::exception::write(self, buffer, self.crashing_thread_context)
            .map_err(WriterError::from)
    }
}

impl Default for MinidumpWriter {
    fn default() -> Self {
        Self::new()
    }
}
