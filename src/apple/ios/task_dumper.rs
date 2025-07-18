// iOS-specific TaskDumper implementation

use crate::apple::common::{mach, ImageInfo, TaskDumpError, TaskDumperBase};
use mach2::mach_types as mt;

/// Wraps a mach call in a Result
macro_rules! mach_call {
    ($call:expr) => {{
        // SAFETY: syscall
        let kr = unsafe { $call };
        if kr == mach::KERN_SUCCESS {
            Ok(())
        } else {
            // This is ugly, improvements to the macro welcome!
            let mut syscall = stringify!($call);
            if let Some(i) = syscall.find('(') {
                syscall = &syscall[..i];
            }
            Err(TaskDumpError::Kernel {
                syscall,
                error: kr.into(),
            })
        }
    }};
}

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

    /// Read the thread list for the task with proper memory management
    pub fn read_threads(&self) -> Result<Vec<mt::thread_t>, TaskDumpError> {
        self.check_current_process()?;

        // SAFETY: We're passing valid pointers to task_threads
        let mut thread_list: mt::thread_array_t = std::ptr::null_mut();
        let mut thread_count: mach2::message::mach_msg_type_number_t = 0;

        mach_call!(mach2::task::task_threads(
            self.base.task,
            &mut thread_list,
            &mut thread_count
        ))?;

        if thread_list.is_null() || thread_count == 0 {
            return Ok(Vec::new());
        }

        // SAFETY: The kernel allocated this memory and gave us the count
        let threads = unsafe { std::slice::from_raw_parts(thread_list, thread_count as usize) };

        // Copy the threads to our own Vec
        let thread_vec = threads.to_vec();

        // CRITICAL FIX: Deallocate the kernel-allocated memory to prevent memory leak
        // SAFETY: We're deallocating memory that was allocated by task_threads
        let _res = mach_call!(mach::mach_vm_deallocate(
            mach::mach_task_self(),
            thread_list as u64,
            (thread_count as u64) * std::mem::size_of::<mt::thread_t>() as u64
        ));

        Ok(thread_vec)
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
