use super::*;
use crate::apple::common::streams::misc_info::{self, TaskDumperHelper};

impl TaskDumperHelper for TaskDumper {
    fn pid_for_task(&self) -> Result<i32, String> {
        self.pid_for_task().map_err(|e| e.to_string())
    }

    fn task_info<T: mach::TaskInfo>(&self) -> Result<T, String> {
        self.task_info().map_err(|e| e.to_string())
    }
}

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
        misc_info::write_misc_info(dumper, buffer).map_err(WriterError::MemoryWriterError)
    }
}
