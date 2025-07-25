// Apple platform common module (macOS, iOS, and future Apple platforms)

pub mod common;

#[cfg(target_os = "macos")]
pub mod mac;

#[cfg(any(
    target_os = "ios",
    all(target_os = "macos", feature = "test-ios-on-macos")
))]
pub mod ios;
