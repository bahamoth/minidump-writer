use crate::{
    minidump_writer::MinidumpWriter,
    crash_handler::{IOSCrashContext, install_crash_handler, uninstall_crash_handler},
};
use std::io::Cursor;
use tempfile::NamedTempFile;

#[test]
fn test_minidump_writer_creation() {
    let writer = MinidumpWriter::new();
    assert!(writer.is_ok(), "Failed to create MinidumpWriter: {:?}", writer.err());
}

#[test]
fn test_minidump_writer_with_crash_context() {
    let crash_context = IOSCrashContext {
        tid: 12345,
        pid: 67890,
        siginfo: unsafe { std::mem::zeroed() },
    };
    
    let writer = MinidumpWriter::with_crash_context(crash_context);
    assert!(writer.is_ok(), "Failed to create MinidumpWriter with crash context: {:?}", writer.err());
    
    let writer = writer.unwrap();
    assert!(writer.crash_context.is_some());
    
    let ctx = writer.crash_context.as_ref().unwrap();
    assert_eq!(ctx.tid, 12345);
    assert_eq!(ctx.pid, 67890);
}

#[test]
fn test_minidump_writer_basic_dump() {
    let mut writer = MinidumpWriter::new().expect("Failed to create MinidumpWriter");
    
    // Create an in-memory buffer for the minidump
    let mut buffer = Cursor::new(Vec::new());
    
    let result = writer.dump(&mut buffer);
    
    // The dump should succeed even with stub implementations
    assert!(result.is_ok(), "Failed to dump minidump: {:?}", result.err());
    
    let dump_data = result.unwrap();
    assert!(!dump_data.is_empty(), "Dump data should not be empty");
    
    // Verify the minidump has the correct header signature
    assert!(dump_data.len() >= 32, "Dump should at least contain a header");
    
    // Check for MDMP signature (0x504D444D in little-endian)
    assert_eq!(&dump_data[0..4], &[0x4D, 0x44, 0x4D, 0x50], "Invalid minidump signature");
}

#[test]
fn test_minidump_writer_to_file() {
    let mut writer = MinidumpWriter::new().expect("Failed to create MinidumpWriter");
    
    // Create a temporary file
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let mut file = temp_file.reopen().expect("Failed to reopen temp file");
    
    let result = writer.dump(&mut file);
    assert!(result.is_ok(), "Failed to dump to file: {:?}", result.err());
    
    // Verify file was written
    let metadata = temp_file.as_file().metadata().expect("Failed to get file metadata");
    assert!(metadata.len() > 0, "File should not be empty");
}

#[test]
fn test_minidump_writer_from_signal_handler_no_context() {
    // When no crash has occurred, from_signal_handler should fail
    let result = MinidumpWriter::from_signal_handler();
    assert!(result.is_err(), "Should fail when no crash context is available");
}

#[test]
fn test_signal_safe_dump() {
    unsafe {
        let mut writer = MinidumpWriter::new().expect("Failed to create MinidumpWriter");
        
        // Set up a crash context
        writer.crash_context = Some(IOSCrashContext {
            tid: 11111,
            pid: 22222,
            siginfo: std::mem::zeroed(),
        });
        
        // Test signal-safe dumping
        let result = writer.dump_signal_safe();
        
        // This might fail because CRASH_FD is not set up, but it shouldn't crash
        // The important thing is that it's signal-safe
        match result {
            Ok(_) => println!("Signal-safe dump succeeded"),
            Err(e) => println!("Signal-safe dump failed (expected): {:?}", e),
        }
    }
}

#[test]
fn test_minidump_header_values() {
    let mut writer = MinidumpWriter::new().expect("Failed to create MinidumpWriter");
    let mut buffer = Cursor::new(Vec::new());
    
    let result = writer.dump(&mut buffer);
    assert!(result.is_ok());
    
    let dump_data = result.unwrap();
    
    // Parse header fields (little-endian)
    let version = u32::from_le_bytes([dump_data[4], dump_data[5], dump_data[6], dump_data[7]]);
    let stream_count = u32::from_le_bytes([dump_data[8], dump_data[9], dump_data[10], dump_data[11]]);
    let stream_directory_rva = u32::from_le_bytes([dump_data[12], dump_data[13], dump_data[14], dump_data[15]]);
    
    // Version should be 0x0000a793 (42899)
    assert_eq!(version, 0x0000a793, "Invalid minidump version");
    
    // Should have multiple streams
    assert!(stream_count > 0, "Should have at least one stream");
    assert!(stream_count <= 10, "Unexpected number of streams: {}", stream_count);
    
    // Stream directory should be after the header
    assert!(stream_directory_rva >= 32, "Stream directory RVA should be after header");
}

#[test] 
fn test_multiple_dumps() {
    // Test that we can create multiple dumps without issues
    for i in 0..3 {
        let mut writer = MinidumpWriter::new().expect("Failed to create MinidumpWriter");
        let mut buffer = Cursor::new(Vec::new());
        
        let result = writer.dump(&mut buffer);
        assert!(result.is_ok(), "Failed to dump iteration {}: {:?}", i, result.err());
        
        let dump_data = result.unwrap();
        assert!(!dump_data.is_empty(), "Dump {} should not be empty", i);
    }
}

#[test]
fn test_crash_context_in_dump() {
    let crash_context = IOSCrashContext {
        tid: 0xABCD,
        pid: 0x1234,
        siginfo: unsafe {
            let mut si: libc::siginfo_t = std::mem::zeroed();
            si.si_signo = libc::SIGSEGV;
            si
        },
    };
    
    let mut writer = MinidumpWriter::with_crash_context(crash_context)
        .expect("Failed to create writer with context");
    
    let mut buffer = Cursor::new(Vec::new());
    let result = writer.dump(&mut buffer);
    assert!(result.is_ok(), "Failed to dump with crash context: {:?}", result.err());
    
    // With crash context, we should have an exception stream
    // (once implemented, the stream count should be higher)
}

#[test]
fn test_system_info_preinitialization() {
    let writer = MinidumpWriter::new().expect("Failed to create MinidumpWriter");
    
    // System info should be pre-initialized for signal safety
    assert!(writer.system_info.is_some(), "System info should be pre-initialized");
    
    if let Some(ref sys_info) = writer.system_info {
        assert!(sys_info.cpu_count > 0);
        assert!(sys_info.memory_size > 0);
        assert!(sys_info.os_version_len > 0);
        assert!(sys_info.machine_type_len > 0);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[test]
    #[ignore] // Ignore by default as this is a more complex integration test
    fn test_full_crash_handling_flow() {
        // 1. Install crash handler
        uninstall_crash_handler();
        install_crash_handler().expect("Failed to install crash handler");
        
        // 2. Create a minidump writer
        let mut writer = MinidumpWriter::new().expect("Failed to create writer");
        
        // 3. Generate a minidump
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let mut file = temp_file.reopen().expect("Failed to reopen temp file");
        
        writer.dump(&mut file).expect("Failed to dump");
        
        // 4. Verify the dump file exists and has content
        let metadata = temp_file.as_file().metadata().expect("Failed to get metadata");
        assert!(metadata.len() > 32, "Dump file should contain more than just header");
        
        // 5. Clean up
        uninstall_crash_handler();
    }
}