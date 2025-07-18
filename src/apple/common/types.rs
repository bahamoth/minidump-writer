// Common types shared between Apple platforms

use thiserror::Error;

#[derive(Error, Debug)]
pub enum TaskDumpError {
    #[error("kernel error {syscall} {error}")]
    Kernel {
        syscall: &'static str,
        error: super::mach::KernelError,
    },
    #[error("detected an invalid mach image header")]
    InvalidMachHeader,
    #[error(transparent)]
    NonUtf8String(#[from] std::string::FromUtf8Error),
    #[error(transparent)]
    NonUtf8Str(#[from] std::str::Utf8Error),
    #[error("unable to find the main executable image for the process")]
    NoExecutableImage,
    #[error("expected load command {name}({id:?}) was not found for an image")]
    MissingLoadCommand {
        name: &'static str,
        id: super::mach::LoadCommandKind,
    },
    #[error("iOS security restriction: {0}")]
    SecurityRestriction(String),
}

/// `dyld_all_image_infos` from <usr/include/mach-o/dyld_images.h>
///
/// This struct is truncated as we only need a couple of fields at the beginning
/// of the struct
#[repr(C)]
#[derive(Copy, Clone)]
pub struct AllImagesInfo {
    // VERSION 1
    pub version: u32,
    /// The number of [`ImageInfo`] structs at that following address
    pub info_array_count: u32,
    /// The address in the process where the array of [`ImageInfo`] structs is
    pub info_array_addr: u64,
    /// A function pointer, unused
    pub _notification: u64,
    /// Unused
    pub _process_detached_from_shared_region: bool,
    // VERSION 2
    pub lib_system_initialized: bool,
    // Note that crashpad adds a 32-bit int here to get proper alignment when
    // building on 32-bit targets...but we explicitly don't care about 32-bit
    // targets since Apple doesn't
    pub dyld_image_load_address: u64,
}

/// `dyld_image_info` from <usr/include/mach-o/dyld_images.h>
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ImageInfo {
    /// The address in the process where the image is loaded
    pub load_address: u64,
    /// The address in the process where the image's file path can be read
    pub file_path: u64,
    /// Timestamp for when the image's file was last modified
    pub file_mod_date: u64,
}

impl PartialEq for ImageInfo {
    fn eq(&self, o: &Self) -> bool {
        self.load_address == o.load_address
    }
}

impl Eq for ImageInfo {}

impl Ord for ImageInfo {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.load_address.cmp(&o.load_address)
    }
}

impl PartialOrd for ImageInfo {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}

/// Describes a region of virtual memory
pub struct VMRegionInfo {
    pub info: super::mach::vm_region_submap_info_64,
    pub range: std::ops::Range<u64>,
}
