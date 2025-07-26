use crate::apple::common::mach;
use crate::dir_section::DumpBuf;
use crate::mem_writer::{write_string_to_location, MemoryWriter};
use crate::minidump_format::*;

/// iOS-specific error type for system info operations
#[derive(Debug, thiserror::Error)]
pub enum SystemInfoError {
    #[error("Failed to allocate memory for system info")]
    AllocationError,
    #[error("Failed to write system info to buffer")]
    WriteError,
}

/// Retrieve the iOS version information.
fn ios_version() -> (u32, u32, u32) {
    let vers = mach::sysctl_string(b"kern.osproductversion\0");

    let inner = || {
        let mut it = vers.split('.');

        let major: u32 = it.next()?.parse().ok()?;
        let minor: u32 = it.next()?.parse().ok()?;
        let patch: u32 = it.next().and_then(|p| p.parse().ok()).unwrap_or_default();

        Some((major, minor, patch))
    };

    inner().unwrap_or_default()
}

/// Retrieves the iOS build version.
#[inline]
fn build_version() -> String {
    mach::sysctl_string(b"kern.osversion\0")
}

/// Writes the system info stream for iOS.
pub fn write_system_info(buffer: &mut DumpBuf) -> Result<MDRawDirectory, SystemInfoError> {
    let mut info_section = MemoryWriter::<MDRawSystemInfo>::alloc(buffer)
        .map_err(|_| SystemInfoError::AllocationError)?;

    let dirent = MDRawDirectory {
        stream_type: MDStreamType::SystemInfoStream as u32,
        location: info_section.location(),
    };

    let number_of_processors: u8 = mach::int_sysctl_by_name(b"hw.ncpu\0");

    // SAFETY: POD buffer
    let cpu: format::CPU_INFORMATION = unsafe { std::mem::zeroed() };
    // Note: iOS doesn't expose the same CPU features as macOS x86_64

    // Determine processor architecture based on target
    let processor_architecture = if cfg!(ios_simulator) {
        // Simulator can be either x86_64 (Intel Mac) or ARM64 (Apple Silicon Mac)
        if cfg!(target_arch = "x86_64") {
            MDCPUArchitecture::PROCESSOR_ARCHITECTURE_AMD64
        } else {
            MDCPUArchitecture::PROCESSOR_ARCHITECTURE_ARM64_OLD
        }
    } else {
        // Real iOS devices are always ARM64 (or ARM64e for newer devices)
        MDCPUArchitecture::PROCESSOR_ARCHITECTURE_ARM64_OLD
    };

    // Get CPU family information
    let family: u32 = mach::sysctl_by_name(b"hw.cpufamily\0");

    // Extract processor level and revision from family
    let processor_level = ((family & 0xffff0000) >> 16) as u16;
    let processor_revision = (family & 0x0000ffff) as u16;

    let (major_version, minor_version, build_number) = ios_version();
    let os_version_loc = write_string_to_location(buffer, &build_version())
        .map_err(|_| SystemInfoError::WriteError)?;

    let info = MDRawSystemInfo {
        // CPU
        processor_architecture: processor_architecture as u16,
        processor_level,
        processor_revision,
        number_of_processors,
        cpu,

        // OS
        platform_id: PlatformId::Ios as u32,
        product_type: 1, // Mobile device
        major_version,
        minor_version,
        build_number,
        csd_version_rva: os_version_loc.rva,

        suite_mask: 0,
        reserved2: 0,
    };

    info_section
        .set_value(buffer, info)
        .map_err(|_| SystemInfoError::WriteError)?;

    Ok(dirent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ios_version_parsing() {
        // This test would need to be run on an actual iOS device or simulator
        let (major, minor, patch) = ios_version();

        // iOS versions should be reasonable (iOS 12+)
        assert!(major >= 12);
        assert!(minor <= 20); // Reasonable upper bound
        assert!(patch <= 10); // Reasonable upper bound
    }

    #[test]
    fn test_build_version_format() {
        let build = build_version();

        // Build version should not be empty
        assert!(!build.is_empty());

        // iOS build versions typically start with a number
        assert!(build.chars().next().unwrap().is_numeric());
    }

    #[test]
    fn test_processor_count() {
        let count: u8 = mach::int_sysctl_by_name(b"hw.ncpu\0");

        // iOS devices have at least 2 cores since iPhone 5
        assert!(count >= 2);

        // Reasonable upper bound
        assert!(count <= 16);
    }
}
