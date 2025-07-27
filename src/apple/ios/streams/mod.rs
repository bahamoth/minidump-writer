// iOS stream writers

pub mod exception;
pub mod memory_list;
pub mod system_info;
pub mod thread_list;

// Re-export key functions for tests
pub use exception::write as write_exception;
pub use memory_list::write as write_memory_list;
pub use system_info::write_system_info;
pub use thread_list::write as write_thread_list;

// System info stream is not yet implemented for iOS
// pub use system_info::*;

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("Memory writer error: {0}")]
    MemoryWriterError(String),
}
