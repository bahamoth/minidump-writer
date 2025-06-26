#![cfg(all(test, target_os = "ios"))]

/// Tests to verify signal safety and iOS-specific constraints
use std::sync::atomic::{AtomicUsize, Ordering};

// Custom allocator to detect allocations in signal context
static ALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);
static IN_SIGNAL_HANDLER: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
mod allocation_detector {
    use super::*;
    use std::alloc::{GlobalAlloc, Layout, System};

    struct DetectingAllocator;

    unsafe impl GlobalAlloc for DetectingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let in_handler = IN_SIGNAL_HANDLER.load(Ordering::SeqCst);
            if in_handler > 0 {
                ALLOCATION_COUNT.fetch_add(1, Ordering::SeqCst);
                // In real scenario, we'd abort here
                eprintln!("ALLOCATION IN SIGNAL HANDLER DETECTED!");
            }
            System.alloc(layout)
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            System.dealloc(ptr, layout)
        }
    }

    #[global_allocator]
    static ALLOCATOR: DetectingAllocator = DetectingAllocator;
}

#[test]
fn test_no_allocations_in_signal_context() {
    use minidump_writer::crash_handler::SignalSafeWriter;
    
    // Reset counters
    ALLOCATION_COUNT.store(0, Ordering::SeqCst);
    
    // Simulate being in signal handler
    IN_SIGNAL_HANDLER.store(1, Ordering::SeqCst);
    
    unsafe {
        // These operations should not allocate
        let mut writer = SignalSafeWriter::new();
        let data = b"Test data";
        writer.write(data);
        let _written = writer.get_written();
    }
    
    // Exit signal handler simulation
    IN_SIGNAL_HANDLER.store(0, Ordering::SeqCst);
    
    // Verify no allocations occurred
    let alloc_count = ALLOCATION_COUNT.load(Ordering::SeqCst);
    assert_eq!(alloc_count, 0, "Detected {} allocations in signal handler!", alloc_count);
}

#[test]
fn test_crash_context_is_trivially_copyable() {
    use minidump_writer::crash_handler::IOSCrashContext;
    
    // Verify the crash context can be safely copied in signal context
    let ctx1 = IOSCrashContext {
        tid: 123,
        pid: 456,
        siginfo: unsafe { std::mem::zeroed() },
    };
    
    // This must work without allocation
    let ctx2 = ctx1; // Copy
    assert_eq!(ctx1.tid, ctx2.tid);
    assert_eq!(ctx1.pid, ctx2.pid);
    
    // Verify size is reasonable for stack allocation
    assert!(std::mem::size_of::<IOSCrashContext>() < 1024);
}

#[test] 
fn test_pre_allocated_buffers() {
    use minidump_writer::system_info::SignalSafeSystemInfo;
    
    // System info should work without allocations
    IN_SIGNAL_HANDLER.store(1, Ordering::SeqCst);
    ALLOCATION_COUNT.store(0, Ordering::SeqCst);
    
    // This might allocate during creation, but that's OK - it's pre-initialized
    IN_SIGNAL_HANDLER.store(0, Ordering::SeqCst);
    let safe_info = SignalSafeSystemInfo::new().expect("Failed to create system info");
    
    IN_SIGNAL_HANDLER.store(1, Ordering::SeqCst);
    
    // These accesses should not allocate
    let _ = safe_info.cpu_count;
    let _ = safe_info.memory_size;
    let _ = &safe_info.os_version_bytes[0..safe_info.os_version_len];
    
    IN_SIGNAL_HANDLER.store(0, Ordering::SeqCst);
    
    let alloc_count = ALLOCATION_COUNT.load(Ordering::SeqCst);
    assert_eq!(alloc_count, 0, "Detected allocations when accessing pre-allocated buffers!");
}