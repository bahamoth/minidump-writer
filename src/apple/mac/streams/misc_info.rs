use super::*;
use crate::apple::common::streams::misc_info::MiscInfoStream;

impl MiscInfoStream for MinidumpWriter {}

impl MinidumpWriter {
    /// Writes the [`MDStreamType::MiscInfoStream`] stream.
    ///
    /// On MacOS, we write a [`minidump_common::format::MINIDUMP_MISC_INFO_2`]
    /// to this stream, which includes the start time of the process at second
    /// granularity, and the (approximate) amount of time spent in user and
    /// system (kernel) time for the lifetime of the task. We attempt to also
    /// retrieve power ie CPU usage statistics, though this information is only
    /// currently available on x86_64, not aarch64 at the moment.
    pub(crate) fn write_misc_info(
        &mut self,
        buffer: &mut DumpBuf,
        dumper: &TaskDumper,
    ) -> Result<MDRawDirectory, WriterError> {
        MiscInfoStream::write_misc_info(self, buffer, dumper).map_err(WriterError::from)
    }
}
