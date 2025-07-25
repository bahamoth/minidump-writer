// iOS-specific TaskDumper implementation

use crate::apple::common::{mach, ImageInfo, TaskDumpError, TaskDumperBase};
use mach2::mach_types as mt;

/// iOS task dumper for reading process information
///
/// Due to iOS security restrictions, this can only dump the current process.
/// Attempting to dump other processes will fail with security errors.
pub struct TaskDumper {
    base: TaskDumperBase,
}

impl TaskDumper {
    pub fn new(task: mt::task_t) -> Self {
        // On iOS, we can only dump the current task
        let current_task = unsafe { mach2::traps::mach_task_self() };

        if task != current_task {
            // We'll handle this error when attempting operations
            log::warn!("iOS only supports dumping the current process");
        }

        Self {
            base: TaskDumperBase::new(task),
        }
    }

    /// Forward to base implementation
    pub fn read_task_memory<T>(&self, address: u64, count: usize) -> Result<Vec<T>, TaskDumpError>
    where
        T: Sized + Clone,
    {
        self.check_current_process()?;
        self.base.read_task_memory(address, count)
    }

    /// Forward to base implementation
    pub fn read_string(
        &self,
        addr: u64,
        expected_size: Option<usize>,
    ) -> Result<Option<String>, TaskDumpError> {
        self.check_current_process()?;
        self.base.read_string(addr, expected_size)
    }

    /// Forward to base implementation
    pub fn task_info<T: mach::TaskInfo>(&self) -> Result<T, TaskDumpError> {
        self.check_current_process()?;
        self.base.task_info()
    }

    /// Read the thread list for the task
    pub fn read_threads(&self) -> Result<&'static [u32], TaskDumpError> {
        self.check_current_process()?;
        self.base.read_threads()
    }

    /// Read thread state for the specified thread
    pub fn read_thread_state(&self, tid: u32) -> Result<mach::ThreadState, TaskDumpError> {
        self.check_current_process()?;
        let mut thread_state = mach::ThreadState::default();
        mach_call!(mach::thread_get_state(
            tid,
            mach::THREAD_STATE_FLAVOR as i32,
            thread_state.state.as_mut_ptr(),
            &mut thread_state.state_size
        ))?;
        Ok(thread_state)
    }

    /// Get images/modules loaded in the process using dyld API
    /// iOS 14.5+ restricts access to task_info(TASK_DYLD_INFO), so we use dyld APIs directly
    pub fn read_images(&self) -> Result<Vec<ImageInfo>, TaskDumpError> {
        self.check_current_process()?;

        // Use dyld API which is more reliable on iOS
        let count = unsafe { _dyld_image_count() };
        let mut images = Vec::with_capacity(count as usize);

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

            // Create ImageInfo compatible with the common types
            let image = ImageInfo {
                load_address: header_ptr as u64,
                file_path: name_ptr as u64,
                file_mod_date: 0, // Not available via dyld API
            };

            images.push(image);
        }

        Ok(images)
    }

    /// Check if we can access the task
    pub fn can_access_task(&self) -> bool {
        self.base.task == unsafe { mach2::traps::mach_task_self() }
    }

    /// Helper to check if we're accessing the current process
    fn check_current_process(&self) -> Result<(), TaskDumpError> {
        if !self.can_access_task() {
            return Err(TaskDumpError::SecurityRestriction(
                "iOS only supports operations on the current process".into(),
            ));
        }
        Ok(())
    }
}

// dyld API bindings for iOS
extern "C" {
    fn _dyld_image_count() -> u32;
    fn _dyld_get_image_name(image_index: u32) -> *const libc::c_char;
    fn _dyld_get_image_header(image_index: u32) -> *const libc::c_void;
    fn _dyld_get_image_vmaddr_slide(image_index: u32) -> libc::intptr_t;
}
