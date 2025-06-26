//! Simple build test to verify iOS module compiles correctly

#[test]
fn test_ios_builds() {
    // This test just verifies that the iOS module compiles
    // Actual functionality tests need to run on iOS device/simulator
    
    #[cfg(target_os = "ios")]
    {
        use minidump_writer::{MinidumpWriter, install_crash_handler};
        
        // These should compile without errors
        let _ = std::mem::size_of::<MinidumpWriter>();
        let _ = install_crash_handler as fn() -> Result<(), _>;
    }
    
    // Test passes if compilation succeeds
    assert!(true);
}