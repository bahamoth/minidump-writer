// iOS stream writers

pub mod breakpad_info;
pub mod exception;
pub mod memory_list;
pub mod misc_info;
pub mod module_list;
pub mod system_info;
pub mod thread_list;
pub mod thread_names;

// Stream functions are now methods on MinidumpWriter

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("Memory writer error: {0}")]
    MemoryWriterError(String),
}
