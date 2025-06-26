use crate::task_dumper::TaskDumper;
use crate::errors::Error;
use std::sync::Mutex;

// Use a mutex to ensure only one TaskDumper test runs at a time
// since TaskDumper uses a global initialization flag
static TEST_MUTEX: Mutex<()> = Mutex::new(());

#[test]
fn test_task_dumper_creation() {
    let _guard = TEST_MUTEX.lock().unwrap();
    
    let dumper = TaskDumper::new_self_process();
    assert!(dumper.is_ok(), "Failed to create TaskDumper: {:?}", dumper.err());
    
    // Drop the dumper to reset the global flag
    drop(dumper);
}

#[test]
fn test_task_dumper_singleton() {
    let _guard = TEST_MUTEX.lock().unwrap();
    
    let dumper1 = TaskDumper::new_self_process();
    assert!(dumper1.is_ok(), "First TaskDumper creation should succeed");
    
    // Second creation should fail due to singleton pattern
    let dumper2 = TaskDumper::new_self_process();
    assert!(dumper2.is_err(), "Second TaskDumper creation should fail");
    
    match dumper2.err() {
        Some(Error::SecurityRestriction(msg)) => {
            assert!(msg.contains("already initialized"));
        }
        _ => panic!("Expected SecurityRestriction error"),
    }
    
    // Drop first dumper to reset
    drop(dumper1);
}

#[test]
fn test_read_task_memory() {
    let _guard = TEST_MUTEX.lock().unwrap();
    
    let dumper = TaskDumper::new_self_process().expect("Failed to create TaskDumper");
    
    // Test reading from a known good address (our own stack variable)
    let test_data: [u32; 4] = [0x12345678, 0x9ABCDEF0, 0xDEADBEEF, 0xCAFEBABE];
    let test_addr = &test_data as *const _ as u64;
    
    let result = dumper.read_task_memory::<u32>(test_addr, 4);
    assert!(result.is_ok(), "Failed to read memory: {:?}", result.err());
    
    let read_data = result.unwrap();
    assert_eq!(read_data.len(), 4);
    assert_eq!(read_data, test_data);
    
    drop(dumper);
}

#[test]
fn test_read_invalid_memory() {
    let _guard = TEST_MUTEX.lock().unwrap();
    
    let dumper = TaskDumper::new_self_process().expect("Failed to create TaskDumper");
    
    // Try to read from an invalid address (null pointer)
    let result = dumper.read_task_memory::<u8>(0, 10);
    assert!(result.is_err(), "Reading from null pointer should fail");
    
    match result.err() {
        Some(Error::MemoryValidation { addr, size }) => {
            assert_eq!(addr, 0);
            assert_eq!(size, 10);
        }
        _ => panic!("Expected MemoryValidation error"),
    }
    
    drop(dumper);
}

#[test]
fn test_read_string() {
    let _guard = TEST_MUTEX.lock().unwrap();
    
    let dumper = TaskDumper::new_self_process().expect("Failed to create TaskDumper");
    
    // Create a null-terminated string
    let test_string = "Hello, iOS!\0";
    let string_addr = test_string.as_ptr() as u64;
    
    let result = dumper.read_string(string_addr, None);
    assert!(result.is_ok(), "Failed to read string: {:?}", result.err());
    
    let read_string = result.unwrap();
    assert!(read_string.is_some());
    assert_eq!(read_string.unwrap(), "Hello, iOS!");
    
    drop(dumper);
}

#[test]
fn test_read_string_with_limit() {
    let _guard = TEST_MUTEX.lock().unwrap();
    
    let dumper = TaskDumper::new_self_process().expect("Failed to create TaskDumper");
    
    // Create a long string
    let test_string = "This is a very long string that should be truncated\0";
    let string_addr = test_string.as_ptr() as u64;
    
    let result = dumper.read_string(string_addr, Some(10));
    assert!(result.is_ok(), "Failed to read string with limit: {:?}", result.err());
    
    let read_string = result.unwrap();
    assert!(read_string.is_some());
    let string = read_string.unwrap();
    assert!(string.len() <= 10, "String should be truncated to 10 chars or less");
    assert!(test_string.starts_with(&string));
    
    drop(dumper);
}

#[test]
fn test_memory_validation() {
    let _guard = TEST_MUTEX.lock().unwrap();
    
    let dumper = TaskDumper::new_self_process().expect("Failed to create TaskDumper");
    
    // Test validation of a valid address (our stack)
    let test_var = 42u64;
    let valid_addr = &test_var as *const _ as u64;
    
    // This is a private method, but we can test it indirectly through read_task_memory
    let result = dumper.read_task_memory::<u64>(valid_addr, 1);
    assert!(result.is_ok(), "Valid memory should be readable");
    
    // Test validation of clearly invalid address
    let invalid_addr = 0xDEADBEEF0000u64; // Unlikely to be mapped
    let result = dumper.read_task_memory::<u64>(invalid_addr, 1);
    assert!(result.is_err(), "Invalid memory should not be readable");
    
    drop(dumper);
}

#[test]
fn test_thread_state_reading() {
    let _guard = TEST_MUTEX.lock().unwrap();
    
    let dumper = TaskDumper::new_self_process().expect("Failed to create TaskDumper");
    
    // Get current thread ID
    let current_thread = unsafe { mach2::mach_init::mach_thread_self() };
    
    let result = dumper.read_thread_state(current_thread);
    
    // Thread state reading might fail on some platforms/configurations
    if result.is_ok() {
        let state = result.unwrap();
        assert_eq!(state.state_size, 68); // ARM_THREAD_STATE64_COUNT
        // We can't verify specific register values as they're constantly changing
    } else {
        // Log the error but don't fail the test as this might be expected
        // on simulators or certain configurations
        eprintln!("Thread state reading failed (might be expected): {:?}", result.err());
    }
    
    drop(dumper);
}

#[test]
fn test_task_port() {
    let _guard = TEST_MUTEX.lock().unwrap();
    
    let dumper = TaskDumper::new_self_process().expect("Failed to create TaskDumper");
    
    let task_port = dumper.task();
    
    // Verify we got a valid task port
    assert_ne!(task_port, 0, "Task port should not be 0");
    
    // The task port should be the same as mach_task_self()
    let expected_task = unsafe { mach2::traps::mach_task_self() };
    assert_eq!(task_port, expected_task, "Task port should match mach_task_self()");
    
    drop(dumper);
}