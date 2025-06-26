use super::errors::Error;
use libc::{c_char, c_void};
use std::ffi::CStr;

/// iOS-specific system information gathering
#[derive(Debug)]
pub struct IOSSystemInfo {
    pub os_version: String,
    pub machine_type: String,
    pub cpu_count: u32,
    pub memory_size: u64,
}

impl IOSSystemInfo {
    /// Gather system information for iOS device
    /// This is signal-safe as it only uses syscalls and pre-allocated buffers
    pub fn new() -> Result<Self, Error> {
        let os_version = Self::get_os_version()?;
        let machine_type = Self::get_machine_type()?;
        let cpu_count = Self::get_cpu_count()?;
        let memory_size = Self::get_memory_size()?;

        Ok(Self {
            os_version,
            machine_type,
            cpu_count,
            memory_size,
        })
    }

    fn get_os_version() -> Result<String, Error> {
        let mut name = [0u8; 256];
        let mut size = name.len();
        
        // SAFETY: syscall to get kernel version
        let ret = unsafe {
            libc::sysctlbyname(
                "kern.version\0".as_ptr() as *const c_char,
                name.as_mut_ptr() as *mut c_void,
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        };

        if ret != 0 {
            return Err(Error::NotFound);
        }

        // SAFETY: The buffer is null-terminated by sysctlbyname
        let version = unsafe { CStr::from_ptr(name.as_ptr() as *const c_char) };
        Ok(version.to_string_lossy().into_owned())
    }

    fn get_machine_type() -> Result<String, Error> {
        let mut name = [0u8; 256];
        let mut size = name.len();
        
        // SAFETY: syscall to get hardware machine type
        let ret = unsafe {
            libc::sysctlbyname(
                "hw.machine\0".as_ptr() as *const c_char,
                name.as_mut_ptr() as *mut c_void,
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        };

        if ret != 0 {
            return Err(Error::NotFound);
        }

        // SAFETY: The buffer is null-terminated by sysctlbyname
        let machine = unsafe { CStr::from_ptr(name.as_ptr() as *const c_char) };
        Ok(machine.to_string_lossy().into_owned())
    }

    fn get_cpu_count() -> Result<u32, Error> {
        let mut cpu_count: u32 = 0;
        let mut size = std::mem::size_of::<u32>();
        
        // SAFETY: syscall to get CPU count
        let ret = unsafe {
            libc::sysctlbyname(
                "hw.ncpu\0".as_ptr() as *const c_char,
                &mut cpu_count as *mut u32 as *mut c_void,
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        };

        if ret != 0 {
            return Err(Error::NotFound);
        }

        Ok(cpu_count)
    }

    fn get_memory_size() -> Result<u64, Error> {
        let mut mem_size: u64 = 0;
        let mut size = std::mem::size_of::<u64>();
        
        // SAFETY: syscall to get physical memory size
        let ret = unsafe {
            libc::sysctlbyname(
                "hw.memsize\0".as_ptr() as *const c_char,
                &mut mem_size as *mut u64 as *mut c_void,
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        };

        if ret != 0 {
            return Err(Error::NotFound);
        }

        Ok(mem_size)
    }

    /// Get iOS-specific processor information
    pub fn get_processor_info(&self) -> ProcessorInfo {
        ProcessorInfo {
            processor_arch: ProcessorArch::ARM64,
            processor_level: 0, // Not used on ARM
            processor_revision: 0, // Not used on ARM  
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ProcessorArch {
    ARM64 = 12, // MD_CPU_ARCHITECTURE_ARM64
}

#[derive(Debug)]
pub struct ProcessorInfo {
    pub processor_arch: ProcessorArch,
    pub processor_level: u16,
    pub processor_revision: u16,
}

/// Signal-safe system info for use in crash handlers
/// Must be initialized before signal handlers are installed
pub struct SignalSafeSystemInfo {
    pub os_version_bytes: [u8; 256],
    pub os_version_len: usize,
    pub machine_type_bytes: [u8; 256],
    pub machine_type_len: usize,
    pub cpu_count: u32,
    pub memory_size: u64,
}

impl SignalSafeSystemInfo {
    /// Pre-initialize system info for signal-safe access
    pub fn new() -> Result<Self, Error> {
        let info = IOSSystemInfo::new()?;
        
        let mut os_version_bytes = [0u8; 256];
        let os_bytes = info.os_version.as_bytes();
        let os_version_len = os_bytes.len().min(255);
        os_version_bytes[..os_version_len].copy_from_slice(&os_bytes[..os_version_len]);
        
        let mut machine_type_bytes = [0u8; 256];
        let machine_bytes = info.machine_type.as_bytes();
        let machine_type_len = machine_bytes.len().min(255);
        machine_type_bytes[..machine_type_len].copy_from_slice(&machine_bytes[..machine_type_len]);
        
        Ok(Self {
            os_version_bytes,
            os_version_len,
            machine_type_bytes,
            machine_type_len,
            cpu_count: info.cpu_count,
            memory_size: info.memory_size,
        })
    }
}