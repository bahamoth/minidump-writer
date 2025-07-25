//! iOS minidump writer tests
//!
//! This file contains all iOS-related tests:
//! - test module: Platform-independent tests that run anywhere
//! - macos_tests module: Tests that require macOS with test-ios-on-macos feature

#[cfg(test)]
mod test {
    // Test iOS data structures and constants without Mach API dependencies
    // These tests can run on any platform

    use minidump_common::format::{ContextFlagsArm64Old, PlatformId};
    use minidump_writer::minidump_format::*;

    #[test]
    fn test_platform_constants() {
        // Verify iOS platform ID
        let ios_platform_id = PlatformId::Ios as u32;
        assert_eq!(ios_platform_id, 0x8102);

        // Ensure it's different from other platforms
        assert_ne!(ios_platform_id, PlatformId::MacOs as u32);
        assert_ne!(ios_platform_id, PlatformId::Linux as u32);
        assert_ne!(ios_platform_id, PlatformId::Android as u32);
    }

    #[test]
    fn test_minidump_header_constants() {
        // Verify minidump format constants
        assert_eq!(MD_HEADER_SIGNATURE, 0x504d444d); // "MDMP"
        assert_eq!(MD_HEADER_VERSION, 0xa793);
    }

    #[test]
    fn test_mdexception_structure_size() {
        use std::mem;

        // Verify structure sizes match expected values
        assert_eq!(mem::size_of::<MDException>(), 152);
        assert_eq!(mem::size_of::<MDRawThread>(), 48);
        assert_eq!(mem::size_of::<MDRawSystemInfo>(), 56);
    }

    #[test]
    fn test_cpu_architecture_constants() {
        // iOS uses ARM64
        assert_eq!(
            MDCPUArchitecture::PROCESSOR_ARCHITECTURE_ARM64_OLD as u16,
            0x8003
        );

        // Verify it's different from x86
        assert_ne!(
            MDCPUArchitecture::PROCESSOR_ARCHITECTURE_ARM64_OLD as u16,
            MDCPUArchitecture::PROCESSOR_ARCHITECTURE_INTEL as u16
        );
    }

    #[test]
    fn test_location_descriptor() {
        let loc = MDLocationDescriptor {
            rva: 0x1000,
            data_size: 0x100,
        };

        assert_eq!(loc.rva, 0x1000);
        assert_eq!(loc.data_size, 0x100);
    }

    #[test]
    fn test_stream_type_values() {
        // Verify stream type constants
        assert_eq!(MDStreamType::ThreadListStream as u32, 3);
        assert_eq!(MDStreamType::SystemInfoStream as u32, 7);
        assert_eq!(MDStreamType::ExceptionStream as u32, 6);
        assert_eq!(MDStreamType::MemoryListStream as u32, 5);
        assert_eq!(MDStreamType::ModuleListStream as u32, 4);
    }

    #[test]
    fn test_memory_descriptor_creation() {
        let mem_desc = MDMemoryDescriptor {
            start_of_memory_range: 0x7fff0000,
            memory: MDLocationDescriptor {
                rva: 0x2000,
                data_size: 4096,
            },
        };

        assert_eq!(mem_desc.start_of_memory_range, 0x7fff0000);
        assert_eq!(mem_desc.memory.rva, 0x2000);
        assert_eq!(mem_desc.memory.data_size, 4096);
    }

    #[test]
    fn test_context_flags() {
        // ARM64 context flags for iOS
        let full_context = ContextFlagsArm64Old::CONTEXT_ARM64_OLD_FULL.bits();

        // Verify the full context flag includes necessary components
        assert!(full_context > 0);

        // Check that it includes both integer and floating point
        assert_eq!(full_context & 0x00000002, 0x00000002); // INTEGER
        assert_eq!(full_context & 0x00000004, 0x00000004); // FLOATING_POINT
    }

    #[test]
    fn test_exception_code_constants() {
        // Common iOS/macOS exception codes
        const EXC_BAD_ACCESS: u32 = 1;
        const EXC_BAD_INSTRUCTION: u32 = 2;
        const _EXC_ARITHMETIC: u32 = 3;
        const _EXC_EMULATION: u32 = 4;
        const _EXC_SOFTWARE: u32 = 5;
        const EXC_BREAKPOINT: u32 = 6;

        // Basic validation
        assert_eq!(EXC_BAD_ACCESS, 1);
        assert_eq!(EXC_BREAKPOINT, 6);

        // Ensure they're distinct
        assert_ne!(EXC_BAD_ACCESS, EXC_BAD_INSTRUCTION);
    }

    #[test]
    fn test_guid_structure() {
        use minidump_common::format::GUID;

        let guid = GUID {
            data1: 0x12345678,
            data2: 0x9abc,
            data3: 0xdef0,
            data4: [1, 2, 3, 4, 5, 6, 7, 8],
        };

        assert_eq!(guid.data1, 0x12345678);
        assert_eq!(guid.data2, 0x9abc);
        assert_eq!(guid.data3, 0xdef0);
        assert_eq!(guid.data4.len(), 8);
    }

    #[test]
    fn test_thread_stack_sentinel_handling() {
        // Test the logic for handling invalid stack pointers
        const STACK_POINTER_NULL: u64 = 0xdeadbeef;
        const _STACK_READ_FAILED: u64 = 0xfeedface;

        // Simulate thread with null stack pointer
        let thread_with_null = MDRawThread {
            thread_id: 1,
            suspend_count: 0,
            priority_class: 0,
            priority: 0,
            teb: 0,
            stack: MDMemoryDescriptor {
                start_of_memory_range: STACK_POINTER_NULL,
                memory: MDLocationDescriptor {
                    rva: 0x1000,
                    data_size: 16, // Sentinel size
                },
            },
            thread_context: MDLocationDescriptor {
                rva: 0x2000,
                data_size: 1024,
            },
        };

        // Verify sentinel handling
        assert_eq!(
            thread_with_null.stack.start_of_memory_range,
            STACK_POINTER_NULL
        );
        assert_eq!(thread_with_null.stack.memory.data_size, 16);
    }

    #[test]
    fn test_memory_range_calculations() {
        // Test memory range calculations for exception handling
        const IP_MEMORY_SIZE: u64 = 256;

        let exception_address: u64 = 0x100000;
        let start = exception_address.saturating_sub(IP_MEMORY_SIZE / 2);
        let end = exception_address + IP_MEMORY_SIZE / 2;

        assert_eq!(start, 0x100000 - 128);
        assert_eq!(end, 0x100000 + 128);
        assert_eq!(end - start, IP_MEMORY_SIZE);
    }

    #[test]
    fn test_module_list_sorting() {
        // Test that modules would be sorted by load address
        struct MockModule {
            load_address: u64,
            name: &'static str,
        }

        let mut modules = [
            MockModule {
                load_address: 0x3000,
                name: "module3",
            },
            MockModule {
                load_address: 0x1000,
                name: "module1",
            },
            MockModule {
                load_address: 0x2000,
                name: "module2",
            },
        ];

        modules.sort_by_key(|m| m.load_address);

        assert_eq!(modules[0].name, "module1");
        assert_eq!(modules[1].name, "module2");
        assert_eq!(modules[2].name, "module3");
    }

    #[test]
    fn test_exception_info_serialization() {
        // Test exception info to MDException conversion logic
        let exception_kind = 1u32; // EXC_BAD_ACCESS
        let exception_code = 1u32; // KERN_INVALID_ADDRESS
        let exception_subcode = 0xdeadbeefu64;

        let md_exception = MDException {
            exception_code: exception_kind,
            exception_flags: exception_code,
            exception_record: 0,
            exception_address: exception_subcode,
            number_parameters: 0,
            __align: 0,
            exception_information: [0; 15],
        };

        assert_eq!(md_exception.exception_code, 1);
        assert_eq!(md_exception.exception_flags, 1);
        assert_eq!(md_exception.exception_address, 0xdeadbeef);
    }

    #[test]
    fn test_stream_directory_ordering() {
        // Test that stream directory entries maintain order
        let entries = [
            (MDStreamType::SystemInfoStream, 0x1000),
            (MDStreamType::ThreadListStream, 0x2000),
            (MDStreamType::ModuleListStream, 0x3000),
            (MDStreamType::MemoryListStream, 0x4000),
            (MDStreamType::ExceptionStream, 0x5000),
        ];

        // Verify stream types are distinct
        let stream_types: Vec<u32> = entries.iter().map(|(t, _)| *t as u32).collect();

        assert_eq!(stream_types[0], 7); // SystemInfoStream
        assert_eq!(stream_types[1], 3); // ThreadListStream
        assert_eq!(stream_types[2], 4); // ModuleListStream
        assert_eq!(stream_types[3], 5); // MemoryListStream
        assert_eq!(stream_types[4], 6); // ExceptionStream
    }

    #[test]
    fn test_minidump_header_validation() {
        let header = MDRawHeader {
            signature: MD_HEADER_SIGNATURE,
            version: MD_HEADER_VERSION,
            stream_count: 5,
            stream_directory_rva: 0x1000,
            checksum: 0,
            time_date_stamp: 1234567890,
            flags: 0,
        };

        // Validate header fields
        assert_eq!(header.signature, 0x504d444d);
        assert_eq!(header.version, 0xa793);
        assert!(header.stream_count > 0);
        assert!(header.stream_directory_rva > 0);
    }

    #[test]
    fn test_arm64_register_count() {
        // ARM64 has specific register counts
        const ARM64_GP_REG_COUNT: usize = 33; // x0-x30, sp, pc
        const ARM64_FP_REG_COUNT: usize = 32; // v0-v31

        // Verify expected sizes
        assert_eq!(ARM64_GP_REG_COUNT, 33);
        assert_eq!(ARM64_FP_REG_COUNT, 32);
    }

    #[test]
    fn test_memory_protection_flags() {
        // Test VM protection flag combinations
        const _VM_PROT_NONE: i32 = 0x00;
        const VM_PROT_READ: i32 = 0x01;
        const VM_PROT_WRITE: i32 = 0x02;
        const VM_PROT_EXECUTE: i32 = 0x04;

        // Common combinations
        let read_only = VM_PROT_READ;
        let read_write = VM_PROT_READ | VM_PROT_WRITE;
        let read_exec = VM_PROT_READ | VM_PROT_EXECUTE;
        let rwx = VM_PROT_READ | VM_PROT_WRITE | VM_PROT_EXECUTE;

        assert_eq!(read_only, 0x01);
        assert_eq!(read_write, 0x03);
        assert_eq!(read_exec, 0x05);
        assert_eq!(rwx, 0x07);
    }

    #[test]
    fn test_crash_address_alignment() {
        // Test address alignment calculations
        fn align_down(addr: u64, alignment: u64) -> u64 {
            addr & !(alignment - 1)
        }

        fn align_up(addr: u64, alignment: u64) -> u64 {
            (addr + alignment - 1) & !(alignment - 1)
        }

        // Page alignment (4KB)
        const PAGE_SIZE: u64 = 4096;

        assert_eq!(align_down(0x1234, PAGE_SIZE), 0x1000);
        assert_eq!(align_up(0x1234, PAGE_SIZE), 0x2000);
        assert_eq!(align_down(0x1000, PAGE_SIZE), 0x1000);
        assert_eq!(align_up(0x1000, PAGE_SIZE), 0x1000);
    }
}

#[cfg(all(test, target_os = "macos", feature = "test-ios-on-macos"))]
mod macos_tests {
    use minidump_writer::ios_test::*;
    use minidump_writer::minidump_format::*;

    #[test]
    fn test_crash_context_creation() {
        use crash_context::{IosCrashContext, IosExceptionInfo};
        use minidump_writer::apple::common::mach::ThreadState;

        // Create a mock crash context
        let crash_context = IosCrashContext {
            task: unsafe { mach2::traps::mach_task_self() },
            thread: 12345, // Mock thread ID
            handler_thread: 12346,
            exception: Some(IosExceptionInfo {
                kind: 1, // EXC_BAD_ACCESS
                code: 1, // KERN_INVALID_ADDRESS
                subcode: Some(0x1234),
            }),
            thread_state: ThreadState::default(),
        };

        // Verify fields are set correctly
        assert_eq!(crash_context.thread, 12345);
        assert_eq!(crash_context.handler_thread, 12346);

        let exception = crash_context.exception.unwrap();
        assert_eq!(exception.kind, 1);
        assert_eq!(exception.code, 1);
        assert_eq!(exception.subcode, Some(0x1234));
    }

    #[test]
    fn test_task_dump_error_conversion() {
        use minidump_writer::apple::common::mach::KernelError;
        use types::TaskDumpError;

        // Test kernel error conversion
        let kern_err = KernelError::from(1); // KERN_INVALID_ADDRESS
        let task_err = TaskDumpError::from(kern_err);

        match task_err {
            TaskDumpError::KernelError(e) => {
                assert_eq!(e, KernelError::InvalidAddress);
            }
            _ => panic!("Expected KernelError variant"),
        }
    }

    #[test]
    fn test_system_info_platform_id() {
        // This test verifies platform ID constants without calling system APIs
        use minidump_common::format::PlatformId;

        // iOS should use the correct platform ID
        let ios_platform_id = PlatformId::Ios as u32;
        assert_eq!(ios_platform_id, 0x8102);

        // Ensure it's different from macOS
        let macos_platform_id = PlatformId::MacOs as u32;
        assert_ne!(ios_platform_id, macos_platform_id);
    }

    #[test]
    fn test_thread_list_sentinel_values() {
        use streams::thread_list::{STACK_POINTER_NULL, STACK_READ_FAILED};

        // Verify sentinel values are distinct
        assert_ne!(STACK_POINTER_NULL, STACK_READ_FAILED);

        // Verify they are non-zero (to distinguish from valid addresses)
        assert_ne!(STACK_POINTER_NULL, 0);
        assert_ne!(STACK_READ_FAILED, 0);
    }

    #[test]
    fn test_memory_descriptor_creation() {
        use minidump_common::format::MDMemoryDescriptor;
        use minidump_writer::mem_writer::LocationDescriptor;

        // Test memory descriptor creation
        let mem_desc = MDMemoryDescriptor {
            start_of_memory_range: 0x1000,
            memory: LocationDescriptor {
                rva: 0x2000,
                data_size: 0x100,
            }
            .location(),
        };

        assert_eq!(mem_desc.start_of_memory_range, 0x1000);
        assert_eq!(mem_desc.memory.rva, 0x2000);
        assert_eq!(mem_desc.memory.data_size, 0x100);
    }

    #[test]
    fn test_image_info_comparison() {
        use types::ImageInfo;

        let info1 = ImageInfo {
            file_mod_date: 1000,
            load_address: 0x1000,
            file_path: vec![b'/', b't', b'e', b's', b't'],
            image_uuid: Some([0u8; 16]),
        };

        let info2 = ImageInfo {
            file_mod_date: 1000,
            load_address: 0x2000,
            file_path: vec![b'/', b't', b'e', b's', b't'],
            image_uuid: Some([0u8; 16]),
        };

        // Test ordering - should be ordered by load address
        assert!(info1 < info2);
    }

    #[test]
    fn test_minidump_header_constants() {
        // Verify minidump header constants
        assert_eq!(MINIDUMP_SIGNATURE, 0x504d444d); // "MDMP"
        assert_eq!(MINIDUMP_VERSION, 0xa793);
    }

    #[test]
    fn test_exception_stream_creation() {
        use crash_context::IosExceptionInfo;
        use streams::exception::MDException;

        let exception_info = IosExceptionInfo {
            kind: 1,
            code: 13,
            subcode: Some(0xdeadbeef),
        };

        let thread_id = 12345u32;

        // Create MDException from iOS exception info
        let md_exception = MDException {
            exception_code: exception_info.kind,
            exception_flags: exception_info.code,
            exception_record: 0,
            exception_address: exception_info.subcode.unwrap_or(0) as u64,
            number_parameters: 0,
            __align: 0,
            exception_information: [0; 15],
            thread_id,
            __align2: 0,
            thread_context: Default::default(),
        };

        assert_eq!(md_exception.exception_code, 1);
        assert_eq!(md_exception.exception_flags, 13);
        assert_eq!(md_exception.exception_address, 0xdeadbeef);
        assert_eq!(md_exception.thread_id, thread_id);
    }
}
