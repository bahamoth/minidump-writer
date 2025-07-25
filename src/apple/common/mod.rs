// Common code shared between Apple platforms (macOS, iOS)

pub mod mach;
#[macro_use]
pub mod task_dumper_base;

pub(in crate::apple) use task_dumper_base::mach_call;
pub mod types;

pub use task_dumper_base::TaskDumperBase;
pub use types::{AllImagesInfo, ImageInfo, TaskDumpError, VMRegionInfo};
