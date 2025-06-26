use super::errors::Error;
use libc::{c_int, c_void, sigaction, siginfo_t, SIGABRT, SIGBUS, SIGFPE, SIGILL, SIGSEGV};
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::os::raw::c_char;

/// Pre-allocated crash buffer for signal-safe minidump writing
/// 128KB should be sufficient for a basic minidump
pub const CRASH_BUFFER_SIZE: usize = 131072;

/// Static crash buffer - must be pre-allocated for signal safety
static mut CRASH_BUFFER: [u8; CRASH_BUFFER_SIZE] = [0; CRASH_BUFFER_SIZE];

/// File descriptor for crash output
static mut CRASH_FD: c_int = -1;

/// Path for crash file output
static mut CRASH_PATH: [c_char; 256] = [0; 256];

/// Handler installation state
static HANDLER_INSTALLED: AtomicBool = AtomicBool::new(false);

/// Previous signal handlers for chaining
static mut PREVIOUS_HANDLERS: [sigaction; 5] = unsafe { std::mem::zeroed() };

/// Signals we handle
const HANDLED_SIGNALS: [c_int; 5] = [SIGSEGV, SIGBUS, SIGABRT, SIGILL, SIGFPE];

/// iOS crash context structure
#[derive(Clone, Copy)]
pub struct IOSCrashContext {
    pub tid: usize,
    pub pid: usize,
    pub siginfo: siginfo_t,
}

/// Global crash context pointer for signal handler access
static CRASH_CONTEXT: AtomicPtr<IOSCrashContext> = AtomicPtr::new(std::ptr::null_mut());

/// Signal-safe write function that doesn't allocate
unsafe fn write_all_signal_safe(fd: c_int, mut data: &[u8]) {
    while !data.is_empty() {
        let written = libc::write(fd, data.as_ptr() as *const c_void, data.len());
        if written <= 0 {
            break; // Silent failure - we can't do much in signal context
        }
        data = &data[written as usize..];
    }
}

/// The actual signal handler - must be async-signal-safe
unsafe extern "C" fn signal_handler(sig: c_int, info: *mut siginfo_t, _ctx: *mut c_void) {
    // Create crash context if we have the info
    let _crash_ctx = if !info.is_null() {
        let mut context = IOSCrashContext {
            tid: libc::pthread_self() as usize,
            siginfo: std::ptr::read(info),
            pid: libc::getpid() as usize,
        };
        
        // Store the context atomically
        CRASH_CONTEXT.store(&mut context as *mut _, Ordering::SeqCst);
        Some(context)
    } else {
        None
    };

    // Attempt to generate minidump
    if CRASH_FD >= 0 {
        // Write a simple marker to indicate crash dump attempt
        // Actual minidump generation will be implemented in minidump_writer.rs
        let marker = b"iOS Crash Dump\n";
        write_all_signal_safe(CRASH_FD, marker);
        
        // Close the file descriptor
        libc::close(CRASH_FD);
        CRASH_FD = -1;
    }

    // Chain to previous handler if available
    let handler_idx = match sig {
        SIGSEGV => 0,
        SIGBUS => 1,
        SIGABRT => 2,
        SIGILL => 3,
        SIGFPE => 4,
        _ => {
            // Unknown signal, abort
            libc::abort();
        }
    };

    let prev_handler = &PREVIOUS_HANDLERS[handler_idx];
    if prev_handler.sa_sigaction != 0 {
        // Call previous handler
        let handler_fn = std::mem::transmute::<usize, extern "C" fn(c_int, *mut siginfo_t, *mut c_void)>(
            prev_handler.sa_sigaction
        );
        handler_fn(sig, info, _ctx);
    } else {
        // No previous handler or default/ignore, abort
        libc::abort();
    }
}

/// iOS crash handler configuration
#[derive(Clone)]
pub struct IOSCrashConfig {
    /// Pre-allocated buffer size (default: 128KB)
    pub buffer_size: usize,
    /// Custom crash log directory (default: Library/Caches)
    pub crash_directory: Option<String>,
    /// Enable signal handler chaining (default: true)
    pub chain_handlers: bool,
}

impl Default for IOSCrashConfig {
    fn default() -> Self {
        Self {
            buffer_size: CRASH_BUFFER_SIZE,
            crash_directory: None,
            chain_handlers: true,
        }
    }
}

/// Install crash signal handlers for iOS
pub fn install_crash_handler() -> Result<(), Error> {
    install_crash_handler_with_config(&IOSCrashConfig::default())
}

/// Install crash signal handlers with custom configuration
pub fn install_crash_handler_with_config(config: &IOSCrashConfig) -> Result<(), Error> {
    // Check if already installed
    if HANDLER_INSTALLED.swap(true, Ordering::SeqCst) {
        return Err(Error::SecurityRestriction(
            "Crash handler already installed".to_string(),
        ));
    }

    // Set up crash file path
    let crash_dir = config.crash_directory.as_deref().unwrap_or(".");
    
    let crash_path = format!("{}/crash_{}.dmp", crash_dir, std::process::id());
    
    unsafe {
        // Copy path to static buffer
        let path_bytes = crash_path.as_bytes();
        let copy_len = path_bytes.len().min(255);
        CRASH_PATH[..copy_len].copy_from_slice(
            &path_bytes[..copy_len]
                .iter()
                .map(|&b| b as c_char)
                .collect::<Vec<_>>()
        );
        CRASH_PATH[copy_len] = 0; // Null terminate
        
        // Pre-open the crash file descriptor
        CRASH_FD = libc::open(
            std::ptr::addr_of!(CRASH_PATH[0]),
            libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC,
            0o644,
        );
        
        if CRASH_FD < 0 {
            HANDLER_INSTALLED.store(false, Ordering::SeqCst);
            return Err(Error::FileCreation(
                crash_path,
                std::io::Error::last_os_error(),
            ));
        }
    }

    // Install signal handlers
    let mut sa: sigaction = unsafe { std::mem::zeroed() };
    sa.sa_sigaction = signal_handler as usize;
    sa.sa_flags = libc::SA_SIGINFO;
    
    // Block all signals during handler execution
    unsafe {
        libc::sigfillset(&mut sa.sa_mask);
    }

    // Install handlers for each signal
    for (idx, &sig) in HANDLED_SIGNALS.iter().enumerate() {
        unsafe {
            let mut old_sa: sigaction = std::mem::zeroed();
            
            if libc::sigaction(sig, &sa, &mut old_sa) != 0 {
                // Rollback on failure
                uninstall_crash_handler();
                return Err(Error::SignalHandlerInstall);
            }
            
            if config.chain_handlers {
                PREVIOUS_HANDLERS[idx] = old_sa;
            }
        }
    }

    Ok(())
}

/// Uninstall crash handlers and restore previous ones
pub fn uninstall_crash_handler() {
    if !HANDLER_INSTALLED.swap(false, Ordering::SeqCst) {
        return;
    }

    unsafe {
        // Close crash file descriptor
        if CRASH_FD >= 0 {
            libc::close(CRASH_FD);
            CRASH_FD = -1;
        }

        // Restore previous handlers
        for (idx, &sig) in HANDLED_SIGNALS.iter().enumerate() {
            let prev_handler = &PREVIOUS_HANDLERS[idx];
            if prev_handler.sa_sigaction != 0 {
                libc::sigaction(sig, prev_handler, std::ptr::null_mut());
            }
        }
    }
}

/// Get the current crash context if available
pub fn get_crash_context() -> Option<IOSCrashContext> {
    let ptr = CRASH_CONTEXT.load(Ordering::SeqCst);
    if ptr.is_null() {
        None
    } else {
        unsafe { Some(std::ptr::read(ptr)) }
    }
}

/// Signal-safe buffer writer for minidump generation
pub struct SignalSafeWriter {
    buffer: &'static mut [u8],
    position: usize,
}

impl SignalSafeWriter {
    /// Create a new signal-safe writer using the pre-allocated crash buffer
    ///
    /// # Safety
    ///
    /// This function must only be called from a signal handler context where
    /// we have exclusive access to CRASH_BUFFER.
    pub unsafe fn new() -> Self {
        Self {
            buffer: std::slice::from_raw_parts_mut(std::ptr::addr_of_mut!(CRASH_BUFFER[0]), CRASH_BUFFER_SIZE),
            position: 0,
        }
    }

    /// Write data to the buffer
    pub fn write(&mut self, data: &[u8]) -> bool {
        let remaining = self.buffer.len() - self.position;
        if data.len() > remaining {
            return false;
        }
        
        self.buffer[self.position..self.position + data.len()].copy_from_slice(data);
        self.position += data.len();
        true
    }

    /// Get the written data
    pub fn get_written(&self) -> &[u8] {
        &self.buffer[..self.position]
    }

    /// Flush written data to the crash file descriptor
    ///
    /// # Safety
    ///
    /// This function must only be called from a signal handler context where
    /// we have exclusive access to CRASH_FD.
    pub unsafe fn flush_to_fd(&self) -> bool {
        if CRASH_FD >= 0 {
            write_all_signal_safe(CRASH_FD, self.get_written());
            true
        } else {
            false
        }
    }
}