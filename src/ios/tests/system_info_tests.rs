use crate::system_info::{IOSSystemInfo, SignalSafeSystemInfo, ProcessorArch};

#[test]
fn test_ios_system_info_creation() {
    let info = IOSSystemInfo::new();
    assert!(info.is_ok(), "Failed to create IOSSystemInfo: {:?}", info.err());
    
    let info = info.unwrap();
    
    // Verify we got reasonable values
    assert!(!info.os_version.is_empty(), "OS version should not be empty");
    assert!(!info.machine_type.is_empty(), "Machine type should not be empty");
    assert!(info.cpu_count > 0, "CPU count should be greater than 0");
    assert!(info.memory_size > 0, "Memory size should be greater than 0");
    
    // On iOS, machine type should contain "iPhone" or "iPad" or "arm64" for simulators
    let machine_lower = info.machine_type.to_lowercase();
    assert!(
        machine_lower.contains("iphone") || 
        machine_lower.contains("ipad") || 
        machine_lower.contains("arm64") ||
        machine_lower.contains("x86_64"), // Simulator on Intel Mac
        "Unexpected machine type: {}", info.machine_type
    );
}

#[test]
fn test_processor_info() {
    let info = IOSSystemInfo::new().expect("Failed to create IOSSystemInfo");
    let proc_info = info.get_processor_info();
    
    // iOS always runs on ARM64
    assert!(matches!(proc_info.processor_arch, ProcessorArch::ARM64));
    assert_eq!(proc_info.processor_level, 0);
    assert_eq!(proc_info.processor_revision, 0);
}

#[test]
fn test_signal_safe_system_info() {
    let safe_info = SignalSafeSystemInfo::new();
    assert!(safe_info.is_ok(), "Failed to create SignalSafeSystemInfo: {:?}", safe_info.err());
    
    let safe_info = safe_info.unwrap();
    
    // Verify pre-initialized buffers
    assert!(safe_info.os_version_len > 0, "OS version length should be > 0");
    assert!(safe_info.os_version_len <= 256, "OS version length should be <= 256");
    assert!(safe_info.machine_type_len > 0, "Machine type length should be > 0");
    assert!(safe_info.machine_type_len <= 256, "Machine type length should be <= 256");
    
    // Verify the buffers contain valid UTF-8
    let os_version = std::str::from_utf8(&safe_info.os_version_bytes[..safe_info.os_version_len]);
    assert!(os_version.is_ok(), "OS version buffer should contain valid UTF-8");
    
    let machine_type = std::str::from_utf8(&safe_info.machine_type_bytes[..safe_info.machine_type_len]);
    assert!(machine_type.is_ok(), "Machine type buffer should contain valid UTF-8");
    
    // System values should match regular IOSSystemInfo
    let regular_info = IOSSystemInfo::new().expect("Failed to create IOSSystemInfo");
    assert_eq!(safe_info.cpu_count, regular_info.cpu_count);
    assert_eq!(safe_info.memory_size, regular_info.memory_size);
}

#[test]
fn test_system_info_consistency() {
    // Create multiple instances and verify they return consistent values
    let info1 = IOSSystemInfo::new().expect("Failed to create first IOSSystemInfo");
    let info2 = IOSSystemInfo::new().expect("Failed to create second IOSSystemInfo");
    
    // System info should be consistent across instances
    assert_eq!(info1.os_version, info2.os_version);
    assert_eq!(info1.machine_type, info2.machine_type);
    assert_eq!(info1.cpu_count, info2.cpu_count);
    assert_eq!(info1.memory_size, info2.memory_size);
}

#[cfg(not(any(target_os = "ios", all(target_os = "macos", target_arch = "aarch64"))))]
#[test]
#[ignore]
fn test_ios_specific_values() {
    // This test is ignored on non-iOS platforms
    // It would fail on Linux/Windows as they don't have iOS sysctl values
}

#[cfg(any(target_os = "ios", all(target_os = "macos", target_arch = "aarch64")))]
#[test]
fn test_ios_specific_values() {
    let info = IOSSystemInfo::new().expect("Failed to create IOSSystemInfo");
    
    // On actual iOS or Apple Silicon Macs, verify iOS-specific patterns
    let os_version_lower = info.os_version.to_lowercase();
    assert!(
        os_version_lower.contains("darwin") || 
        os_version_lower.contains("ios") ||
        os_version_lower.contains("kernel"),
        "OS version should contain Darwin/iOS/Kernel identifiers: {}", info.os_version
    );
}