use thiserror::Error;

#[derive(Debug, Error)]
pub enum WriterError {
    #[error("Failed to write to buffer")]
    BufferError(#[from] crate::mem_writer::MemoryWriterError),
    
    #[error("Failed to write to file")]
    FileWriteError(#[from] std::io::Error),
    
    #[error("Failed to access task information")]
    TaskAccessError,
    
    #[error("Failed to read thread information")]
    ThreadInfoError,
    
    #[error("Failed to read memory")]
    MemoryReadError,
    
    #[error("Failed to enumerate modules")]
    ModuleEnumerationError,
    
    #[error("iOS security restriction: {0}")]
    SecurityRestriction(String),
    
    #[error("Unsupported operation on iOS: {0}")]
    UnsupportedOperation(String),
}