//! iOS crash context
//!
//! This is a temporary stub implementation until proper iOS support
//! is added to the crash-context crate (tracked as T-013).

use crate::minidump_cpu::RawContextCPU;
use mach2::mach_types::{task_t, thread_t};

/// Information on the exception that caused the crash. This is modeled after
/// the `ExceptionInfo` from the `crash-context` crate for macOS.
#[derive(Copy, Clone, Debug)]
pub struct IosExceptionInfo {
    /// The exception kind, eg. `EXC_BAD_ACCESS`.
    pub kind: u32,
    /// The exception code, eg. `KERN_INVALID_ADDRESS`.
    pub code: u64,
    /// Optional subcode with different meanings depending on the exception type.
    /// For `EXC_BAD_ACCESS` this is the address that was accessed.
    pub subcode: Option<u64>,
}

/// A replacement for the `CrashContext` from the `crash-context` crate, which
/// does not yet support iOS.
#[derive(Debug)]
pub struct IosCrashContext {
    /// The process which crashed.
    pub task: task_t,
    /// The thread in the process that crashed.
    pub thread: thread_t,
    /// The thread that handled the exception. This may be useful to ignore.
    pub handler_thread: thread_t,
    /// Optional exception information.
    pub exception: Option<IosExceptionInfo>,
    /// The CPU context of the crashed thread.
    pub thread_state: crate::apple::common::mach::ThreadState,
}

impl IosCrashContext {
    pub fn fill_cpu_context(&self, cpu: &mut RawContextCPU) {
        self.thread_state.fill_cpu_context(cpu);
    }
}
