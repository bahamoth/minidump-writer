use crate::{
    apple::{
        common::streams::breakpad_info::{self, BreakpadInfoWriter},
        ios::{minidump_writer::MinidumpWriter, task_dumper::TaskDumper},
    },
    dir_section::DumpBuf,
    minidump_format::MDRawDirectory,
};

impl BreakpadInfoWriter for MinidumpWriter {
    fn handler_thread(&self) -> u32 {
        self.handler_thread.unwrap_or(0)
    }

    fn requesting_thread(&self) -> u32 {
        self.crash_context.as_ref().map(|cc| cc.thread).unwrap_or(0)
    }
}

impl MinidumpWriter {
    /// Writes the [`BreakpadInfo`] stream.
    ///
    /// For iOS, the primary use of this stream is to differentiate between
    /// the thread that actually raised an exception, and the thread on which
    /// the exception port was listening, so that the exception port (handler)
    /// thread can be deprioritized/ignored when analyzing the minidump.
    pub(crate) fn write_breakpad_info(
        &mut self,
        buffer: &mut DumpBuf,
        _dumper: &TaskDumper,
    ) -> Result<MDRawDirectory, super::super::WriterError> {
        breakpad_info::write_breakpad_info(self, buffer)
            .map_err(|e| super::super::WriterError::MemoryWriterError(e.to_string()))
    }
}
