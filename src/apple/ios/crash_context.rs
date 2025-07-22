//! iOS crash context
//!
//! This is a temporary stub implementation until proper iOS support
//! is added to the crash-context crate (tracked as T-013).

use mach2::mach_types::{task_t, thread_t};

/// iOS-specific crash context
///
/// This is a placeholder until the crash-context crate supports iOS.
/// Currently provides minimal fields needed for compatibility.
#[derive(Debug, Clone)]
pub struct CrashContext {
    /// The mach task (process) where the crash occurred
    pub task: task_t,
    /// The thread that handled the crash
    pub handler_thread: thread_t,
}

impl CrashContext {
    /// Creates a new crash context for the current process and thread
    pub fn new() -> Self {
        Self {
            // SAFETY: These are system calls to get current task/thread
            task: unsafe { mach2::traps::mach_task_self() },
            handler_thread: unsafe { mach2::mach_init::mach_thread_self() },
        }
    }
}

impl Default for CrashContext {
    fn default() -> Self {
        Self::new()
    }
}
