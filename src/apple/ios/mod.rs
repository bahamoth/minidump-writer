// iOS-specific implementation

mod crash_context;
mod minidump_writer;
pub mod streams;
mod task_dumper;

// iOS-specific exports
pub use crash_context::{IosCrashContext, IosExceptionInfo};
pub use minidump_writer::{MinidumpWriter, WriterError};
pub(crate) use task_dumper::thread_basic_info;
pub use task_dumper::TaskDumper;
