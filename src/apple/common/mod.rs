// Common code shared between Apple platforms (macOS, iOS)

pub mod errors;
pub mod mach;
#[macro_use]
pub mod task_dumper;

pub(in crate::apple) use task_dumper::mach_call;
pub mod streams;
pub mod types;

pub use errors::WriterError;
pub use task_dumper::{TaskDumper, TaskDumperExt};
pub use types::{AllImagesInfo, ImageInfo, TaskDumpError, VMRegionInfo};
// CrashContext and ExceptionInfo are conditionally exported from types module
