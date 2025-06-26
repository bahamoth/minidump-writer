#![cfg(target_os = "ios")]

use minidump_writer::{MinidumpWriter, install_crash_handler, uninstall_crash_handler};
use std::io::Cursor;

#[test]
fn test_ios_minidump_generation() {
    // Create a minidump writer
    let mut writer = MinidumpWriter::new().expect("Failed to create MinidumpWriter");
    
    // Create an in-memory buffer
    let mut buffer = Cursor::new(Vec::new());
    
    // Generate minidump
    let result = writer.dump(&mut buffer);
    assert!(result.is_ok(), "Failed to generate minidump: {:?}", result.err());
    
    let dump_data = result.unwrap();
    
    // Verify basic minidump structure
    assert!(dump_data.len() >= 32, "Minidump too small");
    assert_eq!(&dump_data[0..4], &[0x4D, 0x44, 0x4D, 0x50], "Invalid MDMP signature");
}

#[test]
fn test_ios_crash_handler_lifecycle() {
    // Ensure clean state
    uninstall_crash_handler();
    
    // Install handler
    let install_result = install_crash_handler();
    assert!(install_result.is_ok(), "Failed to install crash handler: {:?}", install_result.err());
    
    // Verify can't install twice
    let second_install = install_crash_handler();
    assert!(second_install.is_err(), "Should not be able to install handler twice");
    
    // Uninstall
    uninstall_crash_handler();
    
    // Should be able to install again
    let reinstall = install_crash_handler();
    assert!(reinstall.is_ok(), "Should be able to reinstall after uninstall");
    
    // Clean up
    uninstall_crash_handler();
}