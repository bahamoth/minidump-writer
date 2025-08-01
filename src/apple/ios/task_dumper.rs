// iOS-specific TaskDumper implementation

use crate::apple::common::mach_call;
use crate::apple::common::{
    mach, AllImagesInfo, ImageInfo, TaskDumpError, TaskDumper, VMRegionInfo,
};
use mach2::mach_types as mt;

/// dyld all image infos version we support
const DYLD_ALL_IMAGE_INFOS_VERSION: u32 = 1;

/// Thread basic info structure for iOS
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct thread_basic_info {
    pub user_time: libc::time_value_t,
    pub system_time: libc::time_value_t,
    pub cpu_usage: libc::integer_t,
    pub policy: libc::policy_t,
    pub run_state: libc::integer_t,
    pub flags: libc::integer_t,
    pub suspend_count: libc::integer_t,
    pub sleep_time: libc::integer_t,
}

/// Implementation of ThreadInfo trait for thread_basic_info
impl mach::ThreadInfo for thread_basic_info {
    /// THREAD_BASIC_INFO
    const FLAVOR: u32 = 3;
}

/// iOS-specific extensions to TaskDumper
///
/// Due to iOS security restrictions, attempting to dump other processes
/// will fail with kernel errors at the system call level.
impl TaskDumper {
    /// Read thread state for the specified thread
    pub fn read_thread_state(&self, tid: u32) -> Result<mach::ThreadState, TaskDumpError> {
        let mut thread_state = mach::ThreadState::default();
        mach_call!(mach::thread_get_state(
            tid,
            mach::THREAD_STATE_FLAVOR as i32,
            thread_state.state.as_mut_ptr(),
            &mut thread_state.state_size
        ))?;
        Ok(thread_state)
    }

    /// Get thread info for the specified thread
    pub fn thread_info<T: mach::ThreadInfo>(&self, tid: u32) -> Result<T, TaskDumpError> {
        let mut thread_info = std::mem::MaybeUninit::<T>::uninit();
        let mut count = (std::mem::size_of::<T>() / std::mem::size_of::<u32>()) as u32;

        mach_call!(mach::thread_info(
            tid,
            T::FLAVOR,
            thread_info.as_mut_ptr().cast(),
            &mut count
        ))?;
        unsafe { Ok(thread_info.assume_init()) }
    }

    /// Get the process ID for the task.
    ///
    /// # iOS Limitations
    /// Can only return PID for the current process. Attempting to get PID
    /// for other tasks will fail with SecurityRestriction error.
    pub fn pid_for_task(&self) -> Result<i32, TaskDumpError> {
        // On iOS, we can only get our own PID
        Ok(unsafe { libc::getpid() })
    }

    /// Get images/modules loaded in the process using dyld API
    ///
    /// # iOS Limitations
    /// iOS 14.5+ restricts access to task_info(TASK_DYLD_INFO), so we use dyld APIs directly.
    /// The following AllImagesInfo fields will have sentinel values:
    /// - `info_array_addr`: 0 (dyld API doesn't expose the array address)
    /// - `dyld_image_load_address`: 0 (not available via dyld API)
    /// - Other fields are populated with available data or safe defaults
    pub fn read_images(&self) -> Result<(AllImagesInfo, Vec<ImageInfo>), TaskDumpError> {
        // Use dyld API which is more reliable on iOS
        let count = unsafe { _dyld_image_count() };
        let mut images = Vec::with_capacity(count as usize);

        // Find dyld image if possible
        let mut dyld_load_address = 0u64;

        for i in 0..count {
            let name_ptr = unsafe { _dyld_get_image_name(i) };
            if name_ptr.is_null() {
                continue;
            }

            let header_ptr = unsafe { _dyld_get_image_header(i) };
            if header_ptr.is_null() {
                continue;
            }

            // Get the slide (ASLR offset) for this image
            let _slide = unsafe { _dyld_get_image_vmaddr_slide(i) };

            // Check if this is dyld
            let name = unsafe { std::ffi::CStr::from_ptr(name_ptr) };
            if let Ok(name_str) = name.to_str() {
                if name_str.contains("/dyld") || name_str.contains("/usr/lib/dyld") {
                    dyld_load_address = header_ptr as u64;
                }
            }

            // Create ImageInfo compatible with the common types
            let image = ImageInfo {
                load_address: header_ptr as u64,
                file_path: name_ptr as u64,
                file_mod_date: 0, // Not available via dyld API
            };

            images.push(image);
        }

        // Create AllImagesInfo with available data
        // Using sentinel value 0 for fields not available on iOS
        let all_images_info = AllImagesInfo {
            version: DYLD_ALL_IMAGE_INFOS_VERSION,
            info_array_count: count,
            info_array_addr: 0, // Not available on iOS (sentinel value)
            _notification: 0,
            _process_detached_from_shared_region: false,
            lib_system_initialized: true, // Assume true for running process
            dyld_image_load_address: dyld_load_address, // May be 0 if not found
        };

        Ok((all_images_info, images))
    }

    /// Find the main executable image
    ///
    /// # Errors
    ///
    /// Returns an error if no executable image (MH_EXECUTE) is found
    pub fn read_executable_image(&self) -> Result<ImageInfo, TaskDumpError> {
        let (_, images) = self.read_images()?;

        for img in images {
            // Read Mach-O header to check file type
            let header_buf = self.read_task_memory::<mach::MachHeader>(img.load_address, 1)?;
            let header = &header_buf[0];

            // Validate magic number before accessing other fields
            if header.magic != mach::MH_MAGIC_64 && header.magic != mach::MH_CIGAM_64 {
                continue; // Skip invalid headers
            }

            if header.file_type == mach::MH_EXECUTE {
                return Ok(img);
            }
        }

        Err(TaskDumpError::NoExecutableImage)
    }

    /// Read load commands for a Mach-O image
    ///
    /// # Errors
    ///
    /// Fails if unable to read the image header or load commands from memory
    pub fn read_load_commands(
        &self,
        image: &ImageInfo,
    ) -> Result<mach::LoadCommands, TaskDumpError> {
        let header_buf = self.read_task_memory::<mach::MachHeader>(image.load_address, 1)?;
        let header = &header_buf[0];

        // Validate magic number
        // iOS runs on ARM64 which is little-endian, but check both for completeness
        if header.magic != mach::MH_MAGIC_64 && header.magic != mach::MH_CIGAM_64 {
            return Err(TaskDumpError::InvalidMachHeader);
        }

        let buffer = self.read_task_memory::<u8>(
            image.load_address + std::mem::size_of::<mach::MachHeader>() as u64,
            header.size_commands as usize,
        )?;

        Ok(mach::LoadCommands {
            buffer,
            count: header.num_commands,
        })
    }

    /// Get VM region info for a specific address
    pub fn get_vm_region(&self, addr: u64) -> Result<VMRegionInfo, TaskDumpError> {
        let mut region_base = addr;
        let mut region_size = 0;
        let mut nesting_level = 0;
        let mut info: mach::vm_region_submap_info_64 = unsafe { std::mem::zeroed() };
        let mut info_size = std::mem::size_of_val(&info) as u32;

        let kr = unsafe {
            mach::mach_vm_region_recurse(
                self.task,
                &mut region_base,
                &mut region_size,
                &mut nesting_level,
                &mut info as *mut _ as *mut i32,
                &mut info_size,
            )
        };

        if kr != mach::KERN_SUCCESS {
            return Err(TaskDumpError::Kernel {
                syscall: "mach_vm_region_recurse",
                error: kr.into(),
            });
        }

        Ok(VMRegionInfo {
            info,
            range: region_base..region_base + region_size,
        })
    }
}

// dyld API bindings for iOS
extern "C" {
    fn _dyld_image_count() -> u32;
    fn _dyld_get_image_name(image_index: u32) -> *const libc::c_char;
    fn _dyld_get_image_header(image_index: u32) -> *const libc::c_void;
    fn _dyld_get_image_vmaddr_slide(image_index: u32) -> libc::intptr_t;
}
