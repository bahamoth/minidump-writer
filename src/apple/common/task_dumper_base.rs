// Base TaskDumper implementation shared between Apple platforms

use super::{mach, types::*};
use mach2::mach_types as mt;

/// Wraps a mach call in a Result
macro_rules! mach_call {
    ($call:expr) => {{
        // SAFETY: syscall
        let kr = unsafe { $call };
        if kr == mach::KERN_SUCCESS {
            Ok(())
        } else {
            // This is ugly, improvements to the macro welcome!
            let mut syscall = stringify!($call);
            if let Some(i) = syscall.find('(') {
                syscall = &syscall[..i];
            }
            Err(TaskDumpError::Kernel {
                syscall,
                error: kr.into(),
            })
        }
    }};
}

pub(crate) use mach_call;

/// Base implementation for TaskDumper with common functionality
/// for all Apple platforms
pub struct TaskDumperBase {
    pub(crate) task: mt::task_t,
    pub(crate) page_size: i64,
}

impl TaskDumperBase {
    /// Constructs a [`TaskDumperBase`] for the specified task
    pub fn new(task: mt::task_t) -> Self {
        Self {
            task,
            // SAFETY: syscall
            page_size: unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as i64,
        }
    }

    /// Reads a block of memory from the task
    ///
    /// # Errors
    ///
    /// The syscall to read the task's memory fails for some reason, eg bad address.
    pub fn read_task_memory<T>(&self, address: u64, count: usize) -> Result<Vec<T>, TaskDumpError>
    where
        T: Sized + Clone,
    {
        let length = (count * std::mem::size_of::<T>()) as u64;

        // use the negative of the page size for the mask to find the page address
        let page_address = address & (-self.page_size as u64);
        let last_page_address =
            (address + length + (self.page_size - 1) as u64) & (-self.page_size as u64);

        let page_size = last_page_address - page_address;
        let mut local_start = 0;
        let mut local_length = 0;

        mach_call!(mach::mach_vm_read(
            self.task,
            page_address,
            page_size,
            &mut local_start,
            &mut local_length
        ))?;

        let mut buffer = Vec::with_capacity(count);

        // SAFETY: this is safe as long as the kernel has not lied to us
        let task_buffer = unsafe {
            std::slice::from_raw_parts(
                (local_start as *const u8)
                    .offset((address - page_address) as isize)
                    .cast(),
                count,
            )
        };
        buffer.extend_from_slice(task_buffer);

        // Don't worry about the return here, if something goes wrong there's probably
        // not much we can do about it, and we have what we want anyways
        let _res = mach_call!(mach::mach_vm_deallocate(
            mach::mach_task_self(),
            local_start as u64, // vm_read returns a pointer, but vm_deallocate takes a integer address :-/
            local_length as u64, // vm_read and vm_deallocate use different sizes :-/
        ));

        Ok(buffer)
    }

    /// Reads a null terminated string starting at the specified address. This
    /// is a specialization of [`read_task_memory`] since strings can span VM
    /// regions.
    ///
    /// If not specified, the string is capped at 8k which should never be close
    /// to being hit in normal scenarios, at least for "system" strings, which is
    /// all this interface is used to retrieve
    ///
    /// # Errors
    ///
    /// Fails if the address cannot be read for some reason, or the string is
    /// not utf-8.
    pub fn read_string(
        &self,
        addr: u64,
        expected_size: Option<usize>,
    ) -> Result<Option<String>, TaskDumpError> {
        // The problem is we don't know how much to read until we know how long
        // the string is. And we don't know how long the string is, until we've read
        // the memory!  So, we'll try to read kMaxStringLength bytes
        // (or as many bytes as we can until we reach the end of the vm region).
        let get_region_size = || -> Result<u64, TaskDumpError> {
            let mut region_base = addr;
            let mut region_size = 0;
            let mut info: mach::vm_region_submap_info_64 = unsafe { std::mem::zeroed() };
            let mut info_size = std::mem::size_of_val(&info) as u32;
            let mut nesting_level = 0;
            let mut kr = mach::KERN_INVALID_ADDRESS;

            while kr != mach::KERN_SUCCESS {
                kr = unsafe {
                    mach::mach_vm_region_recurse(
                        self.task,
                        &mut region_base,
                        &mut region_size,
                        &mut nesting_level,
                        &mut info as *mut _ as *mut i32,
                        &mut info_size,
                    )
                };

                if kr != mach::KERN_SUCCESS {
                    // The kernel will return KERN_INVALID_ADDRESS if the address
                    // is within a unmapped region, or after the end of the
                    // mapped region.  Either way, we can't read here.
                    return Err(TaskDumpError::Kernel {
                        syscall: "mach_vm_region_recurse",
                        error: kr.into(),
                    });
                }

                if info.is_submap != 0 {
                    nesting_level += 1;
                }
            }

            Ok(region_size)
        };

        const MAX_STRING_LENGTH: usize = 8 * 1024;
        let region_size = get_region_size()?;
        let string_offset = addr - (addr & !(self.page_size as u64 - 1));

        let len = std::cmp::min(
            MAX_STRING_LENGTH,
            expected_size.unwrap_or((region_size - string_offset) as usize),
        );

        let string = self.read_task_memory::<u8>(addr, len)?;

        let nul_pos = string.iter().position(|&c| c == 0);
        let string = if let Some(len) = nul_pos {
            &string[0..len]
        } else {
            &string
        };

        if string.is_empty() {
            Ok(None)
        } else {
            Ok(Some(std::str::from_utf8(string)?.to_owned()))
        }
    }

    /// Get basic task info
    pub fn task_info<T: mach::TaskInfo>(&self) -> Result<T, TaskDumpError> {
        let mut info = std::mem::MaybeUninit::<T>::uninit();
        let mut count = (std::mem::size_of::<T>() / std::mem::size_of::<u32>()) as u32;

        mach_call!(mach::task::task_info(
            self.task,
            T::FLAVOR,
            info.as_mut_ptr().cast(),
            &mut count
        ))?;

        // SAFETY: this will be initialized if the call succeeded
        unsafe { Ok(info.assume_init()) }
    }

    /// Retrieves the list of active threads in the target process
    ///
    /// # Errors
    ///
    /// The syscall to retrieve the list of threads fails
    pub fn read_threads(&self) -> Result<&'static [u32], TaskDumpError> {
        let mut threads = std::ptr::null_mut();
        let mut thread_count = 0;

        mach_call!(mach::task_threads(
            self.task,
            &mut threads,
            &mut thread_count
        ))?;

        // SAFETY: the kernel has given us this block of memory with a specific
        // length...trust that it's accurate
        Ok(unsafe { std::slice::from_raw_parts(threads, thread_count as usize) })
    }
}
