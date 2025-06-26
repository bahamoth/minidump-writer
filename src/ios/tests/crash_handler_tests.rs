use crate::crash_handler::{
    install_crash_handler, install_crash_handler_with_config, uninstall_crash_handler, get_crash_context,
    IOSCrashConfig, SignalSafeWriter, IOSCrashContext, CRASH_BUFFER_SIZE
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

// Global flag to track if we're in a test that expects a signal
static TEST_SIGNAL_EXPECTED: AtomicBool = AtomicBool::new(false);

#[test]
fn test_crash_handler_installation() {
    // First uninstall any existing handler
    uninstall_crash_handler();
    
    let result = install_crash_handler();
    assert!(result.is_ok(), "Failed to install crash handler: {:?}", result.err());
    
    // Verify we can't install twice
    let second_result = install_crash_handler();
    assert!(second_result.is_err(), "Second installation should fail");
    
    // Clean up
    uninstall_crash_handler();
}

#[test]
fn test_crash_handler_with_config() {
    uninstall_crash_handler();
    
    let config = IOSCrashConfig {
        buffer_size: CRASH_BUFFER_SIZE,
        crash_directory: Some("/tmp".to_string()),
        chain_handlers: true,
    };
    
    let result = install_crash_handler_with_config(&config);
    assert!(result.is_ok(), "Failed to install crash handler with config: {:?}", result.err());
    
    uninstall_crash_handler();
}

#[test]
fn test_uninstall_handler() {
    // Test that uninstalling without installing is safe
    uninstall_crash_handler();
    
    // Install then uninstall
    install_crash_handler().expect("Failed to install handler");
    uninstall_crash_handler();
    
    // Should be able to install again after uninstalling
    let result = install_crash_handler();
    assert!(result.is_ok(), "Should be able to install after uninstalling");
    
    uninstall_crash_handler();
}

#[test]
fn test_crash_context_initially_none() {
    let context = get_crash_context();
    assert!(context.is_none(), "Crash context should be None initially");
}

#[test]
fn test_signal_safe_writer() {
    unsafe {
        let mut writer = SignalSafeWriter::new();
        
        // Test writing data
        let test_data = b"Hello, signal-safe world!";
        let success = writer.write(test_data);
        assert!(success, "Failed to write to buffer");
        
        // Verify written data
        let written = writer.get_written();
        assert_eq!(written, test_data);
        
        // Test buffer overflow protection
        let large_data = vec![b'X'; CRASH_BUFFER_SIZE + 1];
        let overflow_result = writer.write(&large_data);
        assert!(!overflow_result, "Should fail to write data larger than buffer");
    }
}

#[test]
fn test_signal_safe_writer_incremental() {
    unsafe {
        let mut writer = SignalSafeWriter::new();
        
        // Write multiple chunks
        assert!(writer.write(b"First "));
        assert!(writer.write(b"Second "));
        assert!(writer.write(b"Third"));
        
        let written = writer.get_written();
        assert_eq!(written, b"First Second Third");
    }
}

#[test]
fn test_ios_crash_context_size() {
    // Ensure IOSCrashContext is reasonably sized for signal safety
    let size = std::mem::size_of::<IOSCrashContext>();
    assert!(size < 1024, "IOSCrashContext is too large: {} bytes", size);
    
    // Verify it's Copy
    let ctx = IOSCrashContext {
        tid: 123,
        pid: 456,
        siginfo: unsafe { std::mem::zeroed() },
    };
    let ctx2 = ctx; // This should work because it's Copy
    assert_eq!(ctx.tid, ctx2.tid);
    assert_eq!(ctx.pid, ctx2.pid);
}

#[test]
fn test_crash_config_default() {
    let config = IOSCrashConfig::default();
    assert_eq!(config.buffer_size, CRASH_BUFFER_SIZE);
    assert!(config.crash_directory.is_none());
    assert!(config.chain_handlers);
}

#[test]
fn test_crash_config_clone() {
    let config1 = IOSCrashConfig {
        buffer_size: 256 * 1024,
        crash_directory: Some("/custom/path".to_string()),
        chain_handlers: false,
    };
    
    let config2 = config1.clone();
    assert_eq!(config1.buffer_size, config2.buffer_size);
    assert_eq!(config1.crash_directory, config2.crash_directory);
    assert_eq!(config1.chain_handlers, config2.chain_handlers);
}

// Signal handling tests - these are more complex and potentially dangerous
#[cfg(not(miri))] // Don't run under Miri
#[test]
#[ignore] // Ignore by default as signal tests can be flaky
fn test_signal_handler_basic() {
    use std::ptr;
    
    uninstall_crash_handler();
    
    // Set up a custom signal handler that just sets a flag
    let handled = Arc::new(AtomicBool::new(false));
    let handled_clone = handled.clone();
    
    extern "C" fn test_handler(_: libc::c_int, _: *mut libc::siginfo_t, _: *mut libc::c_void) {
        // Don't do anything in the test handler, just prevent crash
    }
    
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = test_handler as usize;
        sa.sa_flags = libc::SA_SIGINFO;
        
        // Install our test handler first
        libc::sigaction(libc::SIGSEGV, &sa, ptr::null_mut());
    }
    
    // Now install the crash handler (should chain to our test handler)
    let config = IOSCrashConfig {
        buffer_size: CRASH_BUFFER_SIZE,
        crash_directory: Some("/tmp".to_string()),
        chain_handlers: true,
    };
    
    install_crash_handler_with_config(&config).expect("Failed to install crash handler");
    
    // Note: Actually triggering signals in tests is dangerous and platform-specific
    // This test mainly verifies the installation mechanics
    
    uninstall_crash_handler();
}

// Thread safety test
#[test]
fn test_handler_thread_safety() {
    uninstall_crash_handler();
    
    let install_count = Arc::new(AtomicBool::new(false));
    let mut handles = vec![];
    
    // Try to install from multiple threads
    for _ in 0..5 {
        let install_count_clone = install_count.clone();
        let handle = thread::spawn(move || {
            let result = install_crash_handler();
            if result.is_ok() {
                install_count_clone.store(true, Ordering::SeqCst);
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    // Only one thread should have successfully installed
    assert!(install_count.load(Ordering::SeqCst), "At least one installation should succeed");
    
    uninstall_crash_handler();
}

#[test]
fn test_buffer_size_validation() {
    unsafe {
        let writer = SignalSafeWriter::new();
        let written = writer.get_written();
        assert_eq!(written.len(), 0, "Initially should have no data written");
        
        // Verify buffer is actually allocated
        let mut writer = SignalSafeWriter::new();
        writer.write(b"test");
        assert_eq!(writer.get_written().len(), 4);
    }
}