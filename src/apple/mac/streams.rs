// Stream writers for macOS minidump format

mod breakpad_info;
mod exception;
mod memory_list;
mod misc_info;
mod module_list;
mod system_info;
mod thread_list;
mod thread_names;

use super::{
    errors::WriterError,
    minidump_writer::MinidumpWriter,
    task_dumper::{ImageInfo, TaskDumper},
};
use crate::apple::common::{mach, TaskDumpError};
use crate::{dir_section::DumpBuf, mem_writer::*, minidump_format::*};

