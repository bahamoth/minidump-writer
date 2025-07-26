fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-lib=dylib=dbghelp");
    }

    // Detect iOS simulator target
    if let Ok(target) = std::env::var("TARGET") {
        // iOS simulator targets contain "-sim" suffix
        // Examples:
        // - x86_64-apple-ios-sim
        // - aarch64-apple-ios-sim
        if target.ends_with("-apple-ios-sim") {
            println!("cargo:rustc-cfg=ios_simulator");
        }
    }
}
