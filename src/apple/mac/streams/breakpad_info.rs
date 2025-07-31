use super::*;
use crate::apple::common::streams::breakpad_info::BreakpadInfoStream;

impl BreakpadInfoStream for MinidumpWriter {
    fn handler_thread(&self) -> u32 {
        self.handler_thread
    }

    fn requesting_thread(&self) -> u32 {
        self.crash_context.as_ref().map(|cc| cc.thread).unwrap_or(0)
    }
}

impl MinidumpWriter {
    /// Writes the [`BreakpadInfo`] stream.
    ///
    /// For MacOS the primary use of this stream is to differentiate between
    /// the thread that actually raised an exception, and the thread on which
    /// the exception port was listening, so that the exception port (handler)
    /// thread can be deprioritized/ignored when analyzing the minidump.
    pub(crate) fn write_breakpad_info(
        &mut self,
        buffer: &mut DumpBuf,
        _dumper: &TaskDumper,
    ) -> Result<MDRawDirectory, WriterError> {
        BreakpadInfoStream::write_breakpad_info(self, buffer).map_err(WriterError::from)
    }
}
