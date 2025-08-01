use super::*;
use crate::apple::common::streams::misc_info::MiscInfoStream;

impl MiscInfoStream for MinidumpWriter {}

impl MinidumpWriter {
    /// Writes the [`MDStreamType::MiscInfoStream`] stream.
    ///
    /// On iOS, we write a [`MINIDUMP_MISC_INFO_2`] to this stream, which includes
    /// the start time of the process at second granularity, and the (approximate)
    /// amount of time spent in user and system (kernel) time for the lifetime of
    /// the task. CPU frequency information is limited on iOS.
    pub(crate) fn write_misc_info(
        &mut self,
        buffer: &mut DumpBuf,
        dumper: &TaskDumper,
    ) -> Result<MDRawDirectory, WriterError> {
        MiscInfoStream::write_misc_info(self, buffer, dumper).map_err(WriterError::from)
    }
}
