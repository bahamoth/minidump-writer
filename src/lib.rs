cfg_if::cfg_if! {
    if #[cfg(any(target_os = "linux", target_os = "android"))] {
        mod linux;

        pub use linux::*;
    } else if #[cfg(target_os = "windows")] {
        mod windows;

        pub use windows::*;
    } else if #[cfg(any(target_os = "macos", target_os = "ios"))] {
        // New apple common module
        mod apple;

        // Maintain backward compatibility for macOS
        #[cfg(target_os = "macos")]
        pub mod mac {
            // Re-export from apple::mac to maintain backward compatibility
            pub use crate::apple::mac::*;
        }

        // Export platform-specific implementations
        #[cfg(target_os = "macos")]
        pub use mac::*;

        // Maintain backward compatibility - re-export modules with original names
        #[cfg(target_os = "macos")]
        pub mod minidump_writer {
            pub use crate::apple::mac::MinidumpWriter;
        }

        #[cfg(target_os = "macos")]
        pub mod task_dumper {
            pub use crate::apple::mac::TaskDumper;
        }

        #[cfg(target_os = "ios")]
        pub use apple::ios::*;
    }
}

pub mod dir_section;
pub mod mem_writer;
pub mod minidump_cpu;
pub mod minidump_format;

mod serializers;

failspot::failspot_name! {
    pub enum FailSpotName {
        StopProcess,
        FillMissingAuxvInfo,
        ThreadName,
        SuspendThreads,
        CpuInfoFileOpen,
    }
}
