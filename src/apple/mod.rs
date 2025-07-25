// Apple platform common module (macOS, iOS, and future Apple platforms)

pub mod common;

#[cfg(target_os = "macos")]
pub mod mac;

#[cfg(target_os = "ios")]
pub mod ios;
