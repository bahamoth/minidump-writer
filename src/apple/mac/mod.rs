// macOS-specific implementation

mod minidump_writer;
mod streams;
mod task_dumper;

pub mod errors;

// Re-export mach from common
pub mod mach {
    pub use crate::apple::common::mach::*;
}

// Re-export mach2 for backward compatibility
pub use mach2;

// Re-export public types
pub use minidump_writer::MinidumpWriter;
pub use task_dumper::TaskDumper;
