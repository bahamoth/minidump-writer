// macOS-specific TaskDumper implementation

pub use crate::apple::common::ImageInfo;
use crate::apple::common::{mach, AllImagesInfo, TaskDumpError, TaskDumperBase, VMRegionInfo};
use mach2::mach_types as mt;

/// macOS implementation of TaskDumper
/// Unlike iOS, macOS can dump external processes
pub struct TaskDumper {
    base: TaskDumperBase,
}

impl TaskDumper {
    /// Constructs a [`TaskDumper`] for the specified task
    pub fn new(task: mt::task_t) -> Self {
        Self {
            base: TaskDumperBase::new(task),
        }
    }

    /// Get the task handle
    pub fn task(&self) -> mt::task_t {
        self.base.task
    }

    /// Forward to base implementation
    pub fn read_task_memory<T>(&self, address: u64, count: usize) -> Result<Vec<T>, TaskDumpError>
    where
        T: Sized + Clone,
    {
        self.base.read_task_memory(address, count)
    }

    /// Forward to base implementation
    pub fn read_string(
        &self,
        addr: u64,
        expected_size: Option<usize>,
    ) -> Result<Option<String>, TaskDumpError> {
        self.base.read_string(addr, expected_size)
    }

    /// Forward to base implementation
    pub fn task_info<T: mach::TaskInfo>(&self) -> Result<T, TaskDumpError> {
        self.base.task_info()
    }

    /// Retrieves the list of active threads in the target process
    ///
    /// # Errors
    ///
    /// The syscall to retrieve the list of threads fails
    pub fn read_threads(&self) -> Result<&'static [u32], TaskDumpError> {
        self.base.read_threads()
    }

    /// Retrieves the mapping between PID and task
    ///
    /// # Errors
    ///
    /// The syscall to retrieve the mapping fails
    pub fn pid(&self) -> Result<i32, TaskDumpError> {
        let mut pid = 0;

        // SAFETY: syscall
        mach_call!(mach::pid_for_task(self.base.task, &mut pid))?;

        Ok(pid)
    }

    /// Retrieves all of the images loaded in the task
    ///
    /// # Errors
    ///
    /// The syscall to retrieve the location of the loaded images fails, or
    /// the syscall to read the loaded images from the process memory fails
    pub fn read_images(&self) -> Result<(AllImagesInfo, Vec<ImageInfo>), TaskDumpError> {
        // Retrieve the address at which the list of loaded images is located
        // within the task
        let all_images_addr = {
            let dyld_info = self.task_info::<mach::task_info::task_dyld_info>()?;
            dyld_info.all_image_info_addr
        };

        // Here we make the assumption that dyld loaded at the same address in
        // the crashed process vs. this one.  This is an assumption made in
        // "dyld_debug.c" and is said to be nearly always valid.
        let dyld_all_info_buf =
            self.read_task_memory::<u8>(all_images_addr, std::mem::size_of::<AllImagesInfo>())?;
        // SAFETY: this is fine as long as the kernel isn't lying to us
        let all_images_info: &AllImagesInfo = unsafe { &*(dyld_all_info_buf.as_ptr().cast()) };
        let images = self.read_task_memory::<ImageInfo>(
            all_images_info.info_array_addr,
            all_images_info.info_array_count as usize,
        )?;

        Ok((*all_images_info, images))
    }

    /// Retrieves the main executable image
    ///
    /// Note that this method is currently only used for tests due to deficiencies
    /// in `otool`
    ///
    /// # Errors
    ///
    /// Any of the errors that apply to [`Self::read_images`] apply here, in
    /// addition to not being able to find the main executable image
    pub fn read_executable_image(&self) -> Result<ImageInfo, TaskDumpError> {
        let (_, images) = self.read_images()?;
        for img in images {
            let mach_header = self.read_task_memory::<mach::MachHeader>(img.load_address, 1)?;
            let header = &mach_header[0];

            if header.file_type == mach::MH_EXECUTE {
                return Ok(img);
            }
        }

        Err(TaskDumpError::NoExecutableImage)
    }

    /// Retrieves all of the VM regions in the task
    ///
    /// # Errors
    ///
    /// The syscall to retrieve the VM regions fails
    pub fn read_vm_regions(&self) -> Result<Vec<VMRegionInfo>, TaskDumpError> {
        let mut regions = Vec::new();

        let mut region_base = 0;
        let mut region_size = 0;
        let mut info: mach::vm_region_submap_info_64 = unsafe { std::mem::zeroed() };
        let mut info_size = std::mem::size_of_val(&info) as u32;
        let mut nesting_level = 0;

        loop {
            // SAFETY: syscall
            let kr = unsafe {
                mach::mach_vm_region_recurse(
                    self.base.task,
                    &mut region_base,
                    &mut region_size,
                    &mut nesting_level,
                    &mut info as *mut _ as *mut i32,
                    &mut info_size,
                )
            };

            if kr != mach::KERN_SUCCESS {
                if kr == mach::KERN_INVALID_ADDRESS {
                    // We've reached the end of the tasks VM regions
                    break;
                }

                return Err(TaskDumpError::Kernel {
                    syscall: "mach_vm_region_recurse",
                    error: kr.into(),
                });
            }

            if info.is_submap != 0 {
                nesting_level += 1;
            } else {
                regions.push(VMRegionInfo {
                    info,
                    range: region_base..region_base + region_size,
                });

                region_base += region_size;
            }
        }

        Ok(regions)
    }

    /// Retrieves the VM region that contains the specified address
    ///
    /// # Errors
    ///
    /// The syscall to retrieve the VM region fails, or the address is not present
    /// in any VM region
    pub fn get_vm_region(&self, addr: u64) -> Result<VMRegionInfo, TaskDumpError> {
        let mut region_base = addr;
        let mut region_size = 0;
        let mut info: mach::vm_region_submap_info_64 = unsafe { std::mem::zeroed() };
        let mut info_size = std::mem::size_of_val(&info) as u32;
        let mut nesting_level = 0;
        let mut kr = mach::KERN_INVALID_ADDRESS;

        while kr != mach::KERN_SUCCESS {
            // SAFETY: syscall
            kr = unsafe {
                mach::mach_vm_region_recurse(
                    self.base.task,
                    &mut region_base,
                    &mut region_size,
                    &mut nesting_level,
                    &mut info as *mut _ as *mut i32,
                    &mut info_size,
                )
            };

            if kr != mach::KERN_SUCCESS {
                // The kernel will return KERN_INVALID_ADDRESS if the address
                // is within a unmapped region, or after the end of the
                // mapped region.  Either way, we can't read here.
                return Err(TaskDumpError::Kernel {
                    syscall: "mach_vm_region_recurse",
                    error: kr.into(),
                });
            }

            if info.is_submap != 0 {
                nesting_level += 1;
            }
        }

        Ok(VMRegionInfo {
            info,
            range: region_base..region_base + region_size,
        })
    }

    /// Reads all of the load commands for the specified image in the task
    ///
    /// # Errors
    ///
    /// Fails if we are unable to read the image header and load commands from
    /// the task
    pub fn read_load_commands(
        &self,
        image: &ImageInfo,
    ) -> Result<mach::LoadCommands, TaskDumpError> {
        let header_buf = self.read_task_memory::<mach::MachHeader>(image.load_address, 1)?;
        let header = &header_buf[0];

        let buffer = self.read_task_memory::<u8>(
            image.load_address + std::mem::size_of::<mach::MachHeader>() as u64,
            header.size_commands as usize,
        )?;

        Ok(mach::LoadCommands {
            buffer,
            count: header.num_commands,
        })
    }

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

    /// Get PID for the task
    pub fn pid_for_task(&self) -> Result<i32, TaskDumpError> {
        self.pid()
    }
}
