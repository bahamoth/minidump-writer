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
    task_dumper::TaskDumper,
};
use crate::{dir_section::DumpBuf, mem_writer::*, minidump_format::*};

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("Memory writer error: {0}")]
    MemoryWriterError(String),
}
