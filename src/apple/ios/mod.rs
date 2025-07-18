// iOS-specific implementation

mod task_dumper;

// iOS-specific TaskDumper
pub use task_dumper::TaskDumper;

// For now, iOS doesn't have its own MinidumpWriter implementation
// This is a placeholder that will be implemented later
pub struct MinidumpWriter;

// Error types
#[derive(Debug, thiserror::Error)]
pub enum WriterError {
    #[error("iOS MinidumpWriter not yet implemented")]
    NotImplemented,
}
