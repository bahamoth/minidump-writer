#[cfg(test)]
mod tests {
    use super::super::*;
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
            if thread.stack.start_of_memory_range != 0xdeadbeef
                && thread.stack.start_of_memory_range != 0xdeaddead
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
}
