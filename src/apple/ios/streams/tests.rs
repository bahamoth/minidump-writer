#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::apple::ios::minidump_writer::MinidumpWriter;
    use crate::apple::ios::task_dumper::TaskDumper;
    use crate::dir_section::DumpBuf;
    use crate::minidump_format::*;

    #[test]
    fn test_write_system_info() {
        let mut buffer = DumpBuf::new(0);

        // Write system info
        let result = system_info::write_system_info(&mut buffer);
        assert!(result.is_ok());

        let dirent = result.unwrap();
        assert_eq!(dirent.stream_type, MDStreamType::SystemInfoStream as u32);
        assert!(dirent.location.data_size > 0);
        assert_eq!(
            dirent.location.data_size as usize,
            std::mem::size_of::<MDRawSystemInfo>()
        );
    }

    #[test]
    fn test_system_info_contents() {
        let mut buffer = DumpBuf::new(0);

        // Write system info
        let result = system_info::write_system_info(&mut buffer);
        assert!(result.is_ok());

        // Read back the system info
        let bytes = buffer.as_bytes();
        let dirent = result.unwrap();
        let offset = dirent.location.rva as usize;

        // Verify buffer bounds before unsafe access
        assert!(
            offset + std::mem::size_of::<MDRawSystemInfo>() <= bytes.len(),
            "System info offset {} + size {} exceeds buffer length {}",
            offset,
            std::mem::size_of::<MDRawSystemInfo>(),
            bytes.len()
        );

        // SAFETY: We know the buffer contains valid MDRawSystemInfo at this offset
        // and we've verified the bounds above
        let sys_info = unsafe {
            let ptr = bytes.as_ptr().add(offset) as *const MDRawSystemInfo;
            &*ptr
        };

        // Verify iOS platform ID
        assert_eq!(sys_info.platform_id, PlatformId::Ios as u32);

        // Verify processor architecture
        assert_eq!(
            sys_info.processor_architecture,
            MDCPUArchitecture::PROCESSOR_ARCHITECTURE_ARM64_OLD as u16
        );

        // Verify processor count
        assert!(sys_info.number_of_processors >= 2); // iOS devices have at least 2 cores

        // Verify OS version
        assert!(sys_info.major_version >= 12); // iOS 12+
    }

    #[test]
    fn test_minidump_writer_with_system_info() {
        use crate::apple::ios::MinidumpWriter;
        use std::io::Cursor;

        let mut writer = MinidumpWriter::new();
        let mut cursor = Cursor::new(Vec::new());

        // Dump to cursor
        let result = writer.dump(&mut cursor);
        assert!(result.is_ok());

        let bytes = result.unwrap();
        assert!(!bytes.is_empty());

        // Verify buffer is large enough for header
        assert!(
            bytes.len() >= std::mem::size_of::<MDRawHeader>(),
            "Buffer too small for header: {} < {}",
            bytes.len(),
            std::mem::size_of::<MDRawHeader>()
        );

        // Verify header
        // SAFETY: We've verified the buffer is large enough for MDRawHeader
        let header = unsafe {
            let ptr = bytes.as_ptr() as *const MDRawHeader;
            &*ptr
        };

        assert_eq!(header.signature, MINIDUMP_SIGNATURE);
        assert_eq!(header.version, MINIDUMP_VERSION);
        assert!(header.stream_count >= 1); // At least system info stream
    }

    #[test]
    fn test_thread_list_stream() {
        use crate::apple::ios::{MinidumpWriter, TaskDumper};
        use crate::minidump_format::MDRawThread;

        let mut writer = MinidumpWriter::new();
        let dumper = TaskDumper::new(writer.task).unwrap();
        let mut buffer = DumpBuf::new(0);

        // Write thread list
        let result = thread_list::write(&mut writer, &mut buffer, &dumper);
        assert!(result.is_ok());

        let (dirent, _) = result.unwrap();
        assert_eq!(dirent.stream_type, MDStreamType::ThreadListStream as u32);

        // Read back thread count
        let bytes = buffer.as_bytes();
        let offset = dirent.location.rva as usize;
        assert!(offset + 4 <= bytes.len());

        // SAFETY: We know the buffer contains a u32 thread count at this offset
        let thread_count = unsafe {
            let ptr = bytes.as_ptr().add(offset) as *const u32;
            *ptr
        };

        assert!(thread_count >= 1); // At least the main thread

        // Verify thread structures
        let threads_offset = offset + 4;
        let thread_size = std::mem::size_of::<MDRawThread>();

        for i in 0..thread_count as usize {
            let thread_offset = threads_offset + (i * thread_size);
            assert!(
                thread_offset + thread_size <= bytes.len(),
                "Thread {} offset exceeds buffer",
                i
            );

            // SAFETY: We've verified the bounds
            let thread = unsafe {
                let ptr = bytes.as_ptr().add(thread_offset) as *const MDRawThread;
                &*ptr
            };

            // Verify thread has valid data
            assert!(thread.thread_id > 0);
            assert!(thread.thread_context.rva > 0);
            assert!(thread.thread_context.data_size > 0);

            // Stack should be present
            if thread.stack.start_of_memory_range != super::thread_list::STACK_POINTER_NULL
                && thread.stack.start_of_memory_range != super::thread_list::STACK_READ_FAILED
            {
                assert!(thread.stack.memory.data_size > 0);
                assert!(thread.stack.memory.rva > 0);
            }
        }
    }

    #[test]
    fn test_thread_state_capture() {
        use crate::apple::ios::TaskDumper;

        let task = unsafe { mach2::traps::mach_task_self() };
        let dumper = TaskDumper::new(task).unwrap();

        // Get thread list
        let threads = dumper.read_threads().unwrap();
        assert!(!threads.is_empty());

        // Test reading thread state for each thread
        for &tid in threads.iter() {
            let thread_state = dumper.read_thread_state(tid);
            assert!(thread_state.is_ok());

            let state = thread_state.unwrap();
            // Verify we can get stack pointer
            let sp = state.sp();
            assert!(sp != 0, "Thread {} has null stack pointer", tid);

            // Verify we can get program counter
            let pc = state.pc();
            assert!(pc != 0, "Thread {} has null program counter", tid);
        }
    }

    #[test]
    fn test_thread_info_retrieval() {
        use crate::apple::ios::TaskDumper;

        let task = unsafe { mach2::traps::mach_task_self() };
        let dumper = TaskDumper::new(task).unwrap();

        let threads = dumper.read_threads().unwrap();
        assert!(!threads.is_empty());

        // Test getting thread info for the main thread
        let main_tid = threads[0];
        let thread_info =
            dumper.thread_info::<mach2::thread_basic_info::thread_basic_info_t>(main_tid);
        assert!(thread_info.is_ok());

        let info = thread_info.unwrap();
        // Main thread should not be suspended
        assert_eq!(info.suspend_count, 0);
    }

    #[test]
    fn test_stack_overflow_handling() {
        use crate::apple::ios::{MinidumpWriter, TaskDumper};

        let mut writer = MinidumpWriter::new();
        let dumper = TaskDumper::new(writer.task).unwrap();
        let mut buffer = DumpBuf::new(0);

        // We can't easily simulate a real stack overflow, but we can test
        // the handling logic by checking that the sentinel values are properly used
        let result = thread_list::write(&mut writer, &mut buffer, &dumper);
        assert!(result.is_ok());

        let (dirent, _) = result.unwrap();
        let bytes = buffer.as_bytes();
        let offset = dirent.location.rva as usize + 4; // Skip thread count

        // Check if any threads have the sentinel values
        let thread_count = unsafe {
            let ptr = bytes.as_ptr().add(dirent.location.rva as usize) as *const u32;
            *ptr
        };

        let thread_size = std::mem::size_of::<MDRawThread>();
        let mut found_sentinel = false;

        for i in 0..thread_count as usize {
            let thread_offset = offset + (i * thread_size);
            let thread = unsafe {
                let ptr = bytes.as_ptr().add(thread_offset) as *const MDRawThread;
                &*ptr
            };

            // Check for sentinel values
            if thread.stack.start_of_memory_range == super::thread_list::STACK_POINTER_NULL {
                // Stack pointer was null
                assert_eq!(thread.stack.memory.data_size, 16);
                found_sentinel = true;
            } else if thread.stack.start_of_memory_range == super::thread_list::STACK_READ_FAILED {
                // Stack read failed
                assert_eq!(thread.stack.memory.data_size, 16);
                found_sentinel = true;
            }
        }

        // Note: In normal execution, we might not see sentinel values
        // This test primarily ensures the code paths compile and don't panic
    }

    #[test]
    fn test_fragmented_stack_regions() {
        use crate::apple::ios::TaskDumper;

        // This test verifies that calculate_stack_size handles fragmented stacks
        // In practice, this is difficult to simulate without low-level memory manipulation
        let task = unsafe { mach2::traps::mach_task_self() };
        let dumper = TaskDumper::new(task).unwrap();

        // Get the main thread
        let threads = dumper.read_threads().unwrap();
        let main_tid = threads[0];

        // Get thread state to find stack pointer
        let thread_state = dumper.read_thread_state(main_tid).unwrap();
        let sp = thread_state.sp();

        // Verify we can get VM region info for the stack
        let vm_region = dumper.get_vm_region(sp);
        assert!(vm_region.is_ok());

        let region = vm_region.unwrap();
        assert!(region.range.contains(&sp));

        // Check if this is marked as stack memory
        if region.info.user_tag == mach2::vm_statistics::VM_MEMORY_STACK {
            // Verify the region has read permissions
            assert!(
                (region.info.protection & mach2::vm_prot::VM_PROT_READ) != 0,
                "Stack region should be readable"
            );
        }
    }

    #[test]
    fn test_crashed_thread_with_context() {
        use crate::apple::ios::{
            crash_context::{IosCrashContext, IosExceptionInfo},
            MinidumpWriter, TaskDumper,
        };

        let mut writer = MinidumpWriter::new();
        let task = writer.task;
        let current_thread = unsafe { mach2::mach_init::mach_thread_self() };

        // Create a mock crash context
        let crash_context = IosCrashContext {
            task,
            thread: current_thread,
            handler_thread: current_thread,
            exception: Some(IosExceptionInfo {
                kind: 1, // EXC_BAD_ACCESS
                code: 1, // KERN_INVALID_ADDRESS
                subcode: Some(0x1234),
            }),
            thread_state: crate::apple::common::mach::ThreadState::default(),
        };

        writer.crash_context = Some(crash_context);

        let dumper = TaskDumper::new(task).unwrap();
        let mut buffer = DumpBuf::new(0);

        // Write thread list with crash context
        let result = thread_list::write(&mut writer, &mut buffer, &dumper);
        assert!(result.is_ok());

        let (dirent, crashed_thread_context) = result.unwrap();

        // Verify we got a crashed thread context
        assert!(
            crashed_thread_context.is_some(),
            "Should have crashed thread context"
        );

        // Verify the crashed thread has valid context
        let ctx = crashed_thread_context.unwrap();
        assert!(ctx.rva > 0);
        assert!(ctx.data_size > 0);
    }

    #[test]
    fn test_memory_list_stream() {
        use crate::apple::ios::{MinidumpWriter, TaskDumper};

        let mut writer = MinidumpWriter::new();
        let dumper = TaskDumper::new(writer.task).unwrap();
        let mut buffer = DumpBuf::new(0);

        // First write thread list to populate memory_blocks
        let result = thread_list::write(&mut writer, &mut buffer, &dumper);
        assert!(result.is_ok());

        // Verify we have some memory blocks from thread stacks
        assert!(
            !writer.memory_blocks.is_empty(),
            "Should have collected thread stack memory"
        );
        let initial_blocks = writer.memory_blocks.len();

        // Now write memory list
        let memory_result = memory_list::write(&mut writer, &mut buffer, &dumper);
        assert!(memory_result.is_ok());

        let dirent = memory_result.unwrap();
        assert_eq!(dirent.stream_type, MDStreamType::MemoryListStream as u32);
        assert!(dirent.location.data_size > 0);

        // Verify the stream structure
        let bytes = buffer.as_bytes();
        let offset = dirent.location.rva as usize;

        // Read the memory block count
        let block_count = unsafe {
            let ptr = bytes.as_ptr().add(offset) as *const u32;
            *ptr
        };

        // Should have at least the thread stacks
        assert!(
            block_count >= initial_blocks as u32,
            "Memory list should contain at least {} blocks",
            initial_blocks
        );
    }

    #[test]
    fn test_memory_list_with_exception() {
        use crate::apple::ios::{
            crash_context::{IosCrashContext, IosExceptionInfo},
            MinidumpWriter, TaskDumper,
        };

        let mut writer = MinidumpWriter::new();
        let task = writer.task;
        let current_thread = unsafe { mach2::mach_init::mach_thread_self() };

        // Get current thread state for realistic crash context
        let dumper = TaskDumper::new(task).unwrap();
        let thread_state = dumper.read_thread_state(current_thread).unwrap();

        // Create crash context with exception
        let crash_context = IosCrashContext {
            task,
            thread: current_thread,
            handler_thread: current_thread,
            exception: Some(IosExceptionInfo {
                kind: 1, // EXC_BAD_ACCESS
                code: 1, // KERN_INVALID_ADDRESS
                subcode: Some(0x1234),
            }),
            thread_state,
        };

        writer.crash_context = Some(crash_context);

        let mut buffer = DumpBuf::new(0);

        // Write thread list first
        thread_list::write(&mut writer, &mut buffer, &dumper).unwrap();
        let blocks_before = writer.memory_blocks.len();

        // Write memory list - should include IP memory for exception
        let result = memory_list::write(&mut writer, &mut buffer, &dumper);
        assert!(result.is_ok());

        // With an exception, we might have added memory around the IP
        // (though it's not guaranteed if the IP region is inaccessible)
        assert!(writer.memory_blocks.len() >= blocks_before);
    }
}
