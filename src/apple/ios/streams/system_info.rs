use crate::{
    apple::{
        common::mach,
        ios::{minidump_writer::MinidumpWriter, task_dumper::TaskDumper},
    },
    dir_section::DumpBuf,
    mem_writer::{write_string_to_location, MemoryWriter},
    minidump_format::*,
};

/// iOS-specific error type for system info operations
#[derive(Debug, thiserror::Error)]
pub enum SystemInfoError {
    #[error("Failed to allocate memory for system info")]
    Allocation,
    #[error("Failed to write system info to buffer")]
    Write,
    #[error("Memory writer error: {0}")]
    MemoryWriter(#[from] crate::mem_writer::MemoryWriterError),
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

impl MinidumpWriter {
    /// Writes the system info stream for iOS.
    pub(crate) fn write_system_info(
        &mut self,
        buffer: &mut DumpBuf,
        _dumper: &TaskDumper,
    ) -> Result<MDRawDirectory, super::super::WriterError> {
        self.write_system_info_impl(buffer)
            .map_err(|e| super::super::WriterError::SystemInfoError(e))
    }

    fn write_system_info_impl(
        &self,
        buffer: &mut DumpBuf,
    ) -> Result<MDRawDirectory, SystemInfoError> {
        // Allocate space for MDRawSystemInfo using MemoryWriter
        let mut info_section = MemoryWriter::<MDRawSystemInfo>::alloc(buffer)?;
        let dirent = MDRawDirectory {
            stream_type: MDStreamType::SystemInfoStream as u32,
            location: info_section.location(),
        };

        let number_of_processors: u8 = mach::int_sysctl_by_name(b"hw.ncpu\0");

        // SAFETY: POD buffer
        let cpu: format::CPU_INFORMATION = unsafe { std::mem::zeroed() };

        // Determine processor architecture based on target
        let processor_architecture = if cfg!(target_os = "ios") && cfg!(target_arch = "x86_64") {
            // iOS simulator on Intel Mac
            MDCPUArchitecture::PROCESSOR_ARCHITECTURE_AMD64
        } else {
            // Real iOS devices or ARM64 simulator
            MDCPUArchitecture::PROCESSOR_ARCHITECTURE_ARM64_OLD
        };

        // Get CPU family information
        let family: u32 = mach::sysctl_by_name(b"hw.cpufamily\0");

        // Extract processor level and revision from family
        let processor_level = ((family & 0xffff0000) >> 16) as u16;
        let processor_revision = (family & 0x0000ffff) as u16;

        let (major_version, minor_version, build_number) = ios_version();

        // Write the OS build version string and get its location
        let os_version_loc = write_string_to_location(buffer, &build_version())?;

        // Create the system info structure following Microsoft's official layout
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

        // Write the struct using MemoryWriter which handles serialization via scroll
        info_section.set_value(buffer, info)?;

        Ok(dirent)
    }
}
