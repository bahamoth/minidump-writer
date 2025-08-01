use thiserror::Error;

#[derive(Debug, Error)]
pub enum WriterError {
    #[error(transparent)]
    TaskDumpError(#[from] super::TaskDumpError),
    #[error("Failed to write to memory")]
    MemoryWriterError(#[from] crate::mem_writer::MemoryWriterError),
    #[error("Failed to write to file")]
    FileWriterError(#[from] crate::dir_section::FileWriterError),
    #[error("Attempted to write an exception stream with no crash context")]
    NoCrashContext,
    #[error("Stream error: {0}")]
    StreamError(String),
    #[error("System info error: {0}")]
    SystemInfoError(String),
    #[error("Directory error: {0}")]
    DirectoryError(String),
    #[error("IO error")]
    IoError(#[from] std::io::Error),
}
