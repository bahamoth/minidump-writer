// iOS platform implementation for minidump-writer
//
// This module provides iOS-specific minidump generation functionality.
// Due to iOS sandboxing and security restrictions, this implementation
// only supports self-process dumping.

pub mod errors;
pub mod minidump_writer;
pub mod system_info;
pub mod task_dumper;

// Re-export commonly used types
pub use minidump_writer::MinidumpWriter;
pub use system_info::write_system_info;