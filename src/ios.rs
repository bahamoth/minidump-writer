#![allow(unsafe_code)]

pub mod crash_handler;
pub mod errors; 
pub mod minidump_writer;
pub mod system_info;
pub mod task_dumper;

// Public exports following the pattern of other platforms
pub use crash_handler::{install_crash_handler, install_crash_handler_with_config, uninstall_crash_handler, IOSCrashConfig};
pub use minidump_writer::MinidumpWriter;

// Type alias for consistency with other platforms
pub type Pid = i32;