use crate::{
    apple::{
        common::streams::misc_info::{self, TaskDumperHelper},
        ios::{minidump_writer::MinidumpWriter, task_dumper::TaskDumper},
    },
    dir_section::DumpBuf,
    minidump_format::MDRawDirectory,
};

impl TaskDumperHelper for TaskDumper {
    fn pid_for_task(&self) -> Result<i32, String> {
        self.pid_for_task().map_err(|e| e.to_string())
    }

    fn task_info<T: crate::apple::common::mach::TaskInfo>(&self) -> Result<T, String> {
        self.task_info().map_err(|e| e.to_string())
    }
}

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
    ) -> Result<MDRawDirectory, super::super::WriterError> {
        misc_info::write_misc_info(dumper, buffer)
            .map_err(|e| super::super::WriterError::MemoryWriterError(e.to_string()))
    }
}
