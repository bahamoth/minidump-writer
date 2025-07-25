// iOS stream writers

pub mod exception;
pub mod system_info;
pub mod thread_list;

#[cfg(test)]
mod tests;

pub use system_info::*;

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("Memory writer error: {0}")]
    MemoryWriterError(String),
}
