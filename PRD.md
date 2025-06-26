# Product Requirement Document: iOS Support for minidump-writer

## 1. Executive Summary

### Project Overview
Add iOS platform support to the minidump-writer Rust crate, implementing in-process crash dump collection within iOS platform constraints while maximizing code reuse from the existing macOS implementation.

### Problem Statement
- iOS platform security model prohibits out-of-process crash collection
- Existing minidump-writer lacks iOS support despite supporting other major platforms
- iOS developers need offline crash analysis capabilities for production debugging

### Proposed Solution
Implement iOS support by adapting the existing macOS implementation to iOS constraints, maintaining strict signal-safety requirements and leveraging shared Mach kernel interfaces.

## 2. Technical Background

### iOS Platform Characteristics
- **Security Model**: Strict process isolation, no cross-process memory access
- **API Availability**: Limited subset of macOS APIs, no debugging interfaces
- **Code Signing**: All code must be signed, affecting runtime behavior
- **Memory Protection**: ASLR, code signing, and strict W^X enforcement
- **Sandbox**: File system access limited to app container

### Relationship to macOS Implementation
The iOS implementation can reuse significant portions of the macOS codebase:
- Mach kernel interfaces (task and thread inspection)
- Memory reading primitives
- Basic minidump format generation

Key differences requiring adaptation:
- Self-process only (no `task_for_pid`)
- Different system information availability
- iOS-specific module loading mechanisms

## 3. Implementation Approach

### Architecture Overview
```rust
// iOS module structure following existing patterns
src/
  ios/
    minidump_writer.rs    // Main writer, adapted from mac/
    crash_handler.rs      // Signal-safe crash handling
    task_dumper.rs        // Reuses mac/ with iOS constraints
    system_info.rs        // iOS-specific system information
    errors.rs             // Platform-specific error types
```

### Code Reuse Strategy
1. **Shared Components** (70% reuse from macOS):
   - Mach task/thread enumeration
   - Memory reading via `vm_read_overwrite`
   - Basic stream writers

2. **iOS-Specific Components** (30% new):
   - Signal handler installation
   - iOS system information gathering
   - Sandbox-aware file operations

## 4. Signal-Safety Requirements

### Critical Constraints
All crash handling code MUST be async-signal-safe:

```rust
// Pre-allocated resources pattern
mod crash_handler {
    use std::sync::atomic::{AtomicBool, Ordering};
    
    // Pre-allocated crash buffer
    static mut CRASH_BUFFER: [u8; 131072] = [0; 131072]; // 128KB
    static mut CRASH_FD: RawFd = -1;
    static HANDLER_INSTALLED: AtomicBool = AtomicBool::new(false);
    
    // Signal-safe write without error handling that could allocate
    unsafe fn write_all_signal_safe(fd: RawFd, mut data: &[u8]) {
        while !data.is_empty() {
            let written = libc::write(fd, data.as_ptr() as *const _, data.len());
            if written <= 0 { break; } // Silent failure
            data = &data[written as usize..];
        }
    }
}
```

### Prohibited Operations in Signal Context
- Dynamic memory allocation (`malloc`, `Box::new`, `Vec::push`)
- Mutex/lock acquisition (including `println!`, logging)
- Non-async-signal-safe system calls
- Objective-C runtime calls
- Swift runtime interactions

## 5. Technical Requirements

### Platform Support
- **Minimum iOS Version**: iOS 13.0+ (for modern runtime features)
- **Architectures**: arm64 only (iPhone 6s and later)
- **Build Targets**: `aarch64-apple-ios`
- **Simulator Support**: Best-effort for debugging only

### Core Functionality

#### 1. Crash Handler Installation
```rust
pub fn install_crash_handler() -> Result<(), Error> {
    // Install handlers for critical signals
    install_signal_handler(SIGSEGV)?;
    install_signal_handler(SIGBUS)?;
    install_signal_handler(SIGABRT)?;
    install_signal_handler(SIGILL)?;
    install_signal_handler(SIGFPE)?;
    
    // Optional: Mach exception handler for additional coverage
    install_mach_exception_handler()?;
    Ok(())
}
```

#### 2. Minidump Generation
```rust
impl MinidumpWriter {
    /// Creates a writer for the current process (iOS limitation)
    pub fn new() -> Result<Self> {
        Ok(Self {
            task: unsafe { mach_task_self() },
            handler_thread: unsafe { mach_thread_self() },
            ..Default::default()
        })
    }
    
    /// Writes minidump with iOS-specific adaptations
    pub fn dump(&mut self, destination: &mut dyn Write) -> Result<()> {
        // Reuse macOS implementation with iOS guards
        self.write_header(destination)?;
        self.write_thread_list(destination)?;
        self.write_memory_list(destination)?;
        self.write_system_info_ios(destination)?;
        self.write_module_list_ios(destination)?;
        Ok(())
    }
}
```

### iOS-Specific Adaptations

#### Memory Validation
```rust
// Validate memory readability before access
unsafe fn validate_memory_range(addr: usize, size: usize) -> bool {
    let mut info: vm_region_basic_info_64 = std::mem::zeroed();
    let mut info_size = std::mem::size_of_val(&info) as mach_msg_type_number_t;
    let mut object_name: mach_port_t = 0;
    
    let kr = vm_region_64(
        mach_task_self(),
        &mut (addr as vm_address_t),
        &mut (size as vm_size_t),
        VM_REGION_BASIC_INFO_64,
        &mut info as *mut _ as vm_region_info_t,
        &mut info_size,
        &mut object_name,
    );
    
    kr == KERN_SUCCESS && (info.protection & VM_PROT_READ) != 0
}
```

#### Module Enumeration
```rust
// iOS-specific module enumeration using dyld APIs
unsafe fn enumerate_modules() -> Vec<ModuleInfo> {
    let count = _dyld_image_count();
    let mut modules = Vec::with_capacity(count as usize);
    
    for i in 0..count {
        if let Some(header) = _dyld_get_image_header(i) {
            let slide = _dyld_get_image_vmaddr_slide(i);
            let name = _dyld_get_image_name(i);
            // Process module information...
        }
    }
    modules
}
```

## 6. API Design

### Public API (Consistent with Other Platforms)
```rust
// Primary API matching existing platform patterns
pub use ios::MinidumpWriter;
pub use ios::install_crash_handler;

// iOS-specific extensions
pub mod ios {
    /// Configuration for iOS crash handling
    pub struct IOSConfig {
        /// Pre-allocated buffer size (default: 128KB)
        pub buffer_size: usize,
        /// Enable Mach exception handling (default: true)
        pub use_mach_exceptions: bool,
        /// Custom crash log directory (default: Library/Caches)
        pub crash_directory: Option<PathBuf>,
    }
}
```

### Integration Examples
```rust
// Swift/Objective-C integration via C FFI
#[no_mangle]
pub extern "C" fn minidump_install_handler() -> bool {
    install_crash_handler().is_ok()
}

#[no_mangle]
pub extern "C" fn minidump_write_crash(
    path: *const c_char,
    context: *const crash_context::CrashContext,
) -> bool {
    // Implementation...
}
```

## 7. Testing Strategy

### Unit Tests
```rust
#[cfg(all(test, target_os = "ios"))]
mod tests {
    #[test]
    fn test_signal_safety() {
        // Verify no allocations in signal handlers
        // Use custom allocator to detect violations
    }
    
    #[test]
    fn test_memory_validation() {
        // Test memory range validation
    }
}
```

### Integration Tests
- Controlled crash scenarios (null pointer, stack overflow)
- Signal handler chaining verification
- Memory corruption handling
- File system permission tests

### Device Testing Requirements
- Physical iOS device (arm64)
- Test on minimum supported iOS version
- Performance profiling under memory pressure

## 8. Security and Privacy

### Data Sanitization
- Remove keychain references
- Exclude authentication tokens
- Sanitize environment variables
- Respect iOS Data Protection classes

### App Store Compliance
- Document crash handling in privacy policy
- No private API usage
- Proper entitlements declaration

## 9. Build System Integration

### Cargo.toml Modifications
```toml
[target.'cfg(target_os = "ios")'.dependencies]
libc = "0.2"
mach2 = "0.4"

[features]
ios_simulator = [] # Best-effort simulator support
```

### Conditional Compilation
```rust
cfg_if::cfg_if! {
    if #[cfg(target_os = "ios")] {
        mod ios;
        pub use ios::*;
    } else if #[cfg(target_os = "macos")] {
        // Existing macOS module
    }
}
```

## 10. Limitations and Mitigations

### Known Limitations
| Limitation | Impact | Mitigation |
|------------|---------|------------|
| Self-process only | Cannot dump other apps | Document as iOS constraint |
| Limited system info | Reduced diagnostic data | Collect available alternatives |
| No kernel information | Missing low-level data | Focus on app-level data |
| Simulator differences | Testing limitations | Document simulator gaps |

### Error Recovery
```rust
// Fallback strategy for handler failures
static PREVIOUS_HANDLERS: Mutex<SignalHandlers> = Mutex::new(SignalHandlers::new());

unsafe extern "C" fn signal_handler(sig: c_int, info: *mut siginfo_t, ctx: *mut c_void) {
    // Attempt minidump generation
    if !generate_minidump_signal_safe(sig, info, ctx) {
        // Fallback to previous handler if registered
        if let Some(previous) = PREVIOUS_HANDLERS.get(sig) {
            previous(sig, info, ctx);
        } else {
            // Last resort: abort
            libc::abort();
        }
    }
}
```

## 11. Documentation Requirements

### User Documentation
- iOS integration guide
- Swift/Objective-C examples
- Troubleshooting guide
- Performance considerations

### Developer Documentation
- Signal-safety guidelines
- Platform limitation reference
- Testing procedures
- Code review checklist

## 12. Success Metrics

### Technical Metrics
- Crash capture success rate > 95%
- Memory overhead < 1MB
- Handler installation time < 10ms
- Zero memory allocations in signal context

### Quality Metrics
- No clippy warnings
- 80%+ code coverage
- All tests passing on real devices
- Successful App Store submission

## 13. Maintenance Considerations

### Ongoing Requirements
- iOS beta testing for compatibility
- Annual signal-safety audit
- Performance regression monitoring
- Security update tracking

### Future Enhancements
- Catalyst support consideration
- Vision Pro adaptation
- Enhanced symbolication support
- Crash grouping heuristics

## 14. References

### Technical Resources
- [iOS Debugging Guide](https://developer.apple.com/documentation/xcode/diagnosing-issues-using-crash-reports-and-device-logs)
- [Mach Exception Handling](https://www.mikeash.com/pyblog/friday-qa-2013-01-11-mach-exception-handlers.html)
- [Signal Safety](https://man7.org/linux/man-pages/man7/signal-safety.7.html)

### Related Projects
- PLCrashReporter (reference implementation)
- KSCrash (iOS crash reporting)
- Crashlytics (crash analytics)