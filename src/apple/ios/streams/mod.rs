// iOS stream writers

pub mod breakpad_info;
pub mod exception;
pub mod memory_list;
pub mod misc_info;
pub mod module_list;
pub mod system_info;
pub mod thread_list;
pub mod thread_names;

// Common imports for all stream modules
use super::{
    minidump_writer::{MinidumpWriter, WriterError},
};
use crate::{
    apple::common::TaskDumper,
    dir_section::DumpBuf, 
    minidump_format::*
};

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("Memory writer error: {0}")]
    MemoryWriterError(String),
}
