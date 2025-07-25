use crate::mac::mach;
use mach2::mach_types as mt;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TaskDumpError {
    #[error("kernel error {syscall} {error})")]
    Kernel {
        syscall: &'static str,
        error: mach::KernelError,
    },
    #[error("detected an invalid mach image header")]
    InvalidMachHeader,
    #[error(transparent)]
    NonUtf8String(#[from] std::string::FromUtf8Error),
    #[error("unable to find the main executable image for the process")]
    NoExecutableImage,
    #[error("expected load command {name}({id:?}) was not found for an image")]
    MissingLoadCommand {
        name: &'static str,
        id: mach::LoadCommandKind,
    },
    #[error("iOS security restriction: {0}")]
    SecurityRestriction(String),
}

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
    task: mt::task_t,
    /// Cached thread list to avoid repeated syscalls
    thread_list: Option<Vec<mt::thread_t>>,
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
            task,
            thread_list: None,
        }
    }

    /// Read the thread list for the task
    pub fn read_threads(&self) -> Result<&'static [mt::thread_t], TaskDumpError> {
        if self.task != unsafe { mach2::traps::mach_task_self() } {
            return Err(TaskDumpError::SecurityRestriction(
                "iOS only supports reading threads from the current process".into()
            ));
        }

        // SAFETY: We're passing valid pointers to task_threads
        let mut thread_list: mt::thread_array_t = std::ptr::null_mut();
        let mut thread_count: mach2::mach_types::mach_msg_type_number_t = 0;

        mach_call!(mach2::task::task_threads(
            self.task,
            &mut thread_list,
            &mut thread_count
        ))?;

        if thread_list.is_null() || thread_count == 0 {
            return Ok(&[]);
        }

        // SAFETY: The kernel allocated this memory and gave us the count
        let threads = unsafe {
            std::slice::from_raw_parts(thread_list, thread_count as usize)
        };

        Ok(threads)
    }

    /// Get basic task info
    pub fn task_info(&self) -> Result<mach2::task_info::task_basic_info, TaskDumpError> {
        if self.task != unsafe { mach2::traps::mach_task_self() } {
            return Err(TaskDumpError::SecurityRestriction(
                "iOS only supports reading task info from the current process".into()
            ));
        }

        let mut info: mach2::task_info::task_basic_info = unsafe { std::mem::zeroed() };
        let mut count = mach2::task_info::TASK_BASIC_INFO_COUNT;

        mach_call!(mach2::task_info::task_info(
            self.task,
            mach2::task_info::TASK_BASIC_INFO,
            &mut info as *mut _ as mach2::task_info::task_info_t,
            &mut count
        ))?;

        Ok(info)
    }

    /// Check if we can access the task
    pub fn can_access_task(&self) -> bool {
        self.task == unsafe { mach2::traps::mach_task_self() }
    }
}