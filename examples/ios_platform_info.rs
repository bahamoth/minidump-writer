//! Example showing iOS platform detection including simulator
//!
//! This example demonstrates how minidump-writer detects whether it's running
//! on a real iOS device or in the iOS Simulator.

fn main() {
    println!("iOS Platform Detection Example");
    println!("==============================");

    // Check if we're on iOS
    #[cfg(target_os = "ios")]
    {
        println!("Running on iOS platform");

        // Check if simulator
        #[cfg(ios_simulator)]
        {
            println!("Environment: iOS Simulator");

            #[cfg(target_arch = "x86_64")]
            println!("Architecture: x86_64 (Intel Mac simulator)");

            #[cfg(target_arch = "aarch64")]
            println!("Architecture: ARM64 (Apple Silicon Mac simulator)");
        }

        #[cfg(not(ios_simulator))]
        {
            println!("Environment: Real iOS Device");
            println!("Architecture: ARM64");
        }
    }

    #[cfg(not(target_os = "ios"))]
    {
        println!("Not running on iOS platform");
        println!("Current OS: {}", std::env::consts::OS);
        println!("Current Arch: {}", std::env::consts::ARCH);
    }
}
