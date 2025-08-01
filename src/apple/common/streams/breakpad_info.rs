use crate::{
    dir_section::DumpBuf,
    mem_writer::*,
    minidump_format::{
        format::{BreakpadInfoValid, MINIDUMP_BREAKPAD_INFO as BreakpadInfo},
        MDRawDirectory, MDStreamType,
    },
};

/// Extension trait for writing BreakpadInfo stream
pub trait BreakpadInfoStream {
    /// The thread where the exception port handled the exception
    fn handler_thread(&self) -> u32;

    /// The actual thread where the exception was thrown
    fn requesting_thread(&self) -> u32;

    /// Writes the [`BreakpadInfo`] stream.
    ///
    /// For MacOS the primary use of this stream is to differentiate between
    /// the thread that actually raised an exception, and the thread on which
    /// the exception port was listening, so that the exception port (handler)
    /// thread can be deprioritized/ignored when analyzing the minidump.
    fn write_breakpad_info(
        &self,
        buffer: &mut DumpBuf,
    ) -> Result<MDRawDirectory, MemoryWriterError> {
        let bp_section = MemoryWriter::<BreakpadInfo>::alloc_with_val(
            buffer,
            BreakpadInfo {
                validity: BreakpadInfoValid::DumpThreadId.bits()
                    | BreakpadInfoValid::RequestingThreadId.bits(),
                // The thread where the exception port handled the exception, might
                // be useful to ignore/deprioritize when processing the minidump
                dump_thread_id: self.handler_thread(),
                // The actual thread where the exception was thrown
                requesting_thread_id: self.requesting_thread(),
            },
        )?;

        Ok(MDRawDirectory {
            stream_type: MDStreamType::BreakpadInfoStream as u32,
            location: bp_section.location(),
        })
    }
}
