use mach2::kern_return::{
    kern_return_t, KERN_FAILURE, KERN_INVALID_ADDRESS, KERN_INVALID_ARGUMENT, KERN_PROTECTION_FAILURE,
};
use thiserror::Error;
use crate::mem_writer::MemoryWriterError;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to access file: {0}")]
    FileAccess(String, #[source] std::io::Error),

    #[error("Failed to create file: {0}")]
    FileCreation(String, #[source] std::io::Error),

    #[error("Failed to write to file")]
    FileWrite(#[source] std::io::Error),

    #[error("Failed to read file")]
    FileRead(#[source] std::io::Error),

    #[error("Failed to parse /proc file")]
    ProcParseError,

    #[error("Not found")]
    NotFound,

    #[error("A specific pid was expected, but the current pid {actual} is not {expected}")]
    InvalidPid { expected: u32, actual: u32 },

    #[error("iOS process error")]
    IOSProcessError(#[from] ProcessError),

    #[error("Failed to get thread info")]
    ThreadInfo,

    #[error("Failed to read task memory")]
    TaskMemoryRead,

    #[error("Task suspension failure")]
    TaskSuspend,

    #[error("Signal handler installation failed")]
    SignalHandlerInstall,

    #[error("Mach error: {0}")]
    MachError(kern_return_t),

    #[error("No crash context available")]
    NoCrashContext,

    #[error("Memory validation failed for address {addr:#x} size {size}")]
    MemoryValidation { addr: usize, size: usize },

    #[error("iOS security restriction: {0}")]
    SecurityRestriction(String),

    #[error("Memory writer error")]
    MemoryWriter(#[from] MemoryWriterError),

    #[error("Directory section error")]
    DirSection(String),
}

impl From<crate::dir_section::FileWriterError> for Error {
    fn from(e: crate::dir_section::FileWriterError) -> Self {
        match e {
            crate::dir_section::FileWriterError::IOError(io) => Error::FileWrite(io),
            crate::dir_section::FileWriterError::MemoryWriterError(m) => Error::MemoryWriter(m),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::FileWrite(e)
    }
}

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("Failed to get task for self process")]
    SelfTaskError,

    #[error("iOS does not support cross-process dumping")]
    CrossProcessNotSupported,

    #[error("Mach error code: {0}")]
    MachError(kern_return_t),
}

impl From<kern_return_t> for ProcessError {
    fn from(kr: kern_return_t) -> Self {
        ProcessError::MachError(kr)
    }
}

impl ProcessError {
    pub fn mach_error_string(kr: kern_return_t) -> &'static str {
        match kr {
            KERN_INVALID_ADDRESS => "Invalid address",
            KERN_PROTECTION_FAILURE => "Protection failure",
            KERN_INVALID_ARGUMENT => "Invalid argument",
            KERN_FAILURE => "General failure",
            _ => "Unknown error",
        }
    }
}