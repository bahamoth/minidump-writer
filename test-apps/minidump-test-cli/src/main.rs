use clap::{Parser, Subcommand};
use std::fs::File;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

#[cfg(target_os = "ios")]
use minidump_writer::apple::ios::{IosCrashContext, IosExceptionInfo};

#[derive(Parser)]
#[command(name = "minidump-test-cli")]
#[command(about = "Test CLI for minidump-writer library")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output file path (default: ~/Documents/minidump_TIMESTAMP.dmp on iOS/macOS)
    #[arg(short, long, global = true)]
    output: Option<PathBuf>,

    /// Enable debug output
    #[arg(short, long, global = true)]
    debug: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Trigger a crash of specified type
    Crash {
        /// Type of crash to trigger
        #[arg(value_enum)]
        crash_type: CrashType,
    },
    /// Generate a minidump without crashing
    Dump,
    /// Create multiple threads then generate a dump
    Threads {
        /// Number of threads to create
        #[arg(default_value = "5")]
        count: usize,
    },
}

#[derive(clap::ValueEnum, Clone)]
enum CrashType {
    /// Segmentation fault
    Segfault,
    /// Abort
    Abort,
    /// Illegal instruction
    Illegal,
    /// Bus error
    Bus,
    /// Floating point exception
    Fpe,
    /// Trap
    Trap,
}

fn main() {
    let cli = Cli::parse();

    if cli.debug {
        eprintln!("Debug mode enabled");
        eprintln!("Platform: {}", std::env::consts::OS);
        eprintln!("Architecture: {}", std::env::consts::ARCH);
    }

    match cli.command {
        Commands::Crash { crash_type } => handle_crash(crash_type, cli.output, cli.debug),
        Commands::Dump => handle_dump(cli.output, cli.debug),
        Commands::Threads { count } => handle_threads(count, cli.output, cli.debug),
    }
}

fn handle_crash(crash_type: CrashType, output: Option<PathBuf>, debug: bool) {
    // Set up crash handler first
    setup_crash_handler(output, debug);

    match crash_type {
        CrashType::Segfault => {
            eprintln!("Triggering segmentation fault...");
            unsafe {
                let null_ptr: *mut i32 = std::ptr::null_mut();
                *null_ptr = 42;
            }
        }
        CrashType::Abort => {
            eprintln!("Triggering abort...");
            unsafe {
                libc::abort();
            }
        }
        CrashType::Illegal => {
            eprintln!("Triggering illegal instruction...");
            #[cfg(target_arch = "x86_64")]
            unsafe {
                std::arch::asm!("ud2");
            }
            #[cfg(target_arch = "aarch64")]
            unsafe {
                std::arch::asm!("udf #0");
            }
            #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
            {
                eprintln!("Illegal instruction not implemented for this architecture");
                std::process::exit(1);
            }
        }
        CrashType::Bus => {
            eprintln!("Triggering bus error...");
            unsafe {
                // Misaligned memory access on ARM
                let ptr = 1 as *const u64;
                let _val = *ptr;
            }
        }
        CrashType::Fpe => {
            eprintln!("Triggering floating point exception...");
            unsafe {
                libc::raise(libc::SIGFPE);
            }
        }
        CrashType::Trap => {
            eprintln!("Triggering trap...");
            unsafe {
                libc::raise(libc::SIGTRAP);
            }
        }
    }
}

fn handle_dump(output: Option<PathBuf>, debug: bool) {
    let output_path = output.unwrap_or_else(get_default_output_path);
    
    if debug {
        eprintln!("Generating minidump to: {}", output_path.display());
    }

    let mut file = match File::create(&output_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to create output file: {}", e);
            std::process::exit(1);
        }
    };

    // Get current process info
    #[cfg(target_os = "macos")]
    let task = unsafe { mach2::traps::mach_task_self() };
    
    #[cfg(target_os = "linux")]
    let pid = unsafe { libc::getpid() };

    // Create minidump
    #[cfg(target_os = "macos")]
    {
        let mut writer = minidump_writer::MinidumpWriter::new(Some(task), None);
        match writer.dump(&mut file) {
            Ok(_) => {
                println!("Minidump written to: {}", output_path.display());
            }
            Err(e) => {
                eprintln!("Failed to write minidump: {}", e);
                std::process::exit(1);
            }
        }
    }
    
    #[cfg(target_os = "ios")]
    {
        let mut writer = minidump_writer::apple::ios::MinidumpWriter::new();
        match writer.dump(&mut file) {
            Ok(_) => {
                println!("Minidump written to: {}", output_path.display());
            }
            Err(e) => {
                eprintln!("Failed to write minidump: {}", e);
                std::process::exit(1);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        match minidump_writer::linux::minidump_writer::MinidumpWriter::new(pid, None).dump(&mut file) {
            Ok(_) => {
                println!("Minidump written to: {}", output_path.display());
            }
            Err(e) => {
                eprintln!("Failed to write minidump: {}", e);
                std::process::exit(1);
            }
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "linux")))]
    {
        eprintln!("Platform not supported yet");
        std::process::exit(1);
    }
}

fn handle_threads(count: usize, output: Option<PathBuf>, debug: bool) {
    if debug {
        eprintln!("Creating {} threads...", count);
    }

    let mut handles = vec![];

    // Create threads
    for i in 0..count {
        let handle = thread::Builder::new()
            .name(format!("test-thread-{}", i))
            .spawn(move || {
                // Do some work to ensure thread has stack content
                let mut sum = 0u64;
                for j in 0..1000 {
                    sum = sum.wrapping_add(j);
                }
                
                // Keep thread alive
                thread::sleep(Duration::from_secs(3600));
                sum
            })
            .expect("Failed to create thread");
        
        handles.push(handle);
    }

    // Give threads time to start
    thread::sleep(Duration::from_millis(100));

    if debug {
        eprintln!("All threads created, generating dump...");
    }

    // Generate dump with all threads active
    handle_dump(output, debug);
}

fn get_default_output_path() -> PathBuf {
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    {
        // iOS/macOS: Use Documents directory
        if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(format!("{}/Documents/minidump_{}.dmp", home, timestamp))
        } else {
            PathBuf::from(format!("./minidump_{}.dmp", timestamp))
        }
    }
    
    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
    {
        // Other platforms: Use current directory
        PathBuf::from(format!("./minidump_{}.dmp", timestamp))
    }
}

fn setup_crash_handler(output: Option<PathBuf>, debug: bool) {
    use std::sync::Mutex;
    
    // Store the output path in a static for the signal handler
    static OUTPUT_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
    static DEBUG_MODE: Mutex<bool> = Mutex::new(false);
    
    let output_path = output.unwrap_or_else(get_default_output_path);
    *OUTPUT_PATH.lock().unwrap() = Some(output_path.clone());
    *DEBUG_MODE.lock().unwrap() = debug;
    
    if debug {
        eprintln!("Setting up crash handler, will write to: {}", output_path.display());
    }
    
    // Signal handler function
    extern "C" fn signal_handler(sig: libc::c_int) {
        // Note: This is a signal handler, so we must be very careful about what we do here
        // No heap allocations, no mutex locks (except our pre-existing ones), etc.
        
        let output_path = OUTPUT_PATH.lock().unwrap().clone();
        let debug = *DEBUG_MODE.lock().unwrap();
        
        if let Some(path) = output_path {
            if debug {
                eprintln!("\nCaught signal {}, generating minidump...", sig);
            }
            
            // Create minidump using pre-allocated resources
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            {
                if let Ok(mut file) = std::fs::File::create(&path) {
                    let task = unsafe { mach2::traps::mach_task_self() };
                    
                    // Create crash context for iOS
                    #[cfg(target_os = "ios")]
                    {
                        // Get current thread
                        let thread = unsafe { mach2::mach_init::mach_thread_self() };
                        
                        // Get thread state for the current thread
                        let mut thread_state = minidump_writer::apple::common::mach::ThreadState::default();
                        let mut state_count = thread_state.state.len() as u32;
                        
                        let result = unsafe {
                            mach2::thread_act::thread_get_state(
                                thread,
                                minidump_writer::apple::common::mach::THREAD_STATE_FLAVOR as i32,
                                thread_state.state.as_mut_ptr(),
                                &mut state_count
                            )
                        };
                        
                        if result != 0 {
                            eprintln!("Failed to get thread state: {}", result);
                        }
                        
                        thread_state.state_size = state_count;
                        
                        // Create a crash context
                        let crash_context = IosCrashContext {
                            task,
                            thread,
                            handler_thread: thread,
                            exception: Some(IosExceptionInfo {
                                kind: match sig {
                                    libc::SIGSEGV => 1, // EXC_BAD_ACCESS
                                    libc::SIGABRT => 10, // EXC_CRASH
                                    libc::SIGILL => 2, // EXC_BAD_INSTRUCTION
                                    libc::SIGBUS => 1, // EXC_BAD_ACCESS (bus error)
                                    libc::SIGFPE => 3, // EXC_ARITHMETIC
                                    libc::SIGTRAP => 6, // EXC_BREAKPOINT
                                    _ => 0,
                                },
                                code: sig as u64,
                                subcode: None,
                            }),
                            thread_state,
                        };
                        
                        let mut writer = minidump_writer::apple::ios::MinidumpWriter::new();
                        writer.set_crash_context(crash_context);
                        
                        if let Err(e) = writer.dump(&mut file) {
                            eprintln!("Failed to write crash minidump: {}", e);
                        } else if debug {
                            eprintln!("Crash minidump written to: {}", path.display());
                        }
                    }
                    
                    // For macOS, use regular dump
                    #[cfg(not(target_os = "ios"))]
                    {
                        let mut writer = minidump_writer::minidump_writer::MinidumpWriter::new(Some(task), None);
                        if let Err(e) = writer.dump(&mut file) {
                            eprintln!("Failed to write crash minidump: {}", e);
                        } else if debug {
                            eprintln!("Crash minidump written to: {}", path.display());
                        }
                    }
                }
            }
        }
        
        // Re-raise the signal to get default behavior (core dump, etc.)
        unsafe {
            libc::signal(sig, libc::SIG_DFL);
            libc::raise(sig);
        }
    }
    
    // Install signal handlers
    unsafe {
        libc::signal(libc::SIGSEGV, signal_handler as libc::sighandler_t);
        libc::signal(libc::SIGABRT, signal_handler as libc::sighandler_t);
        libc::signal(libc::SIGILL, signal_handler as libc::sighandler_t);
        libc::signal(libc::SIGBUS, signal_handler as libc::sighandler_t);
        libc::signal(libc::SIGFPE, signal_handler as libc::sighandler_t);
        libc::signal(libc::SIGTRAP, signal_handler as libc::sighandler_t);
    }
}