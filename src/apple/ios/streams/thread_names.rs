use crate::{
    apple::{
        common::mach,
        ios::{minidump_writer::MinidumpWriter, task_dumper::TaskDumper},
    },
    dir_section::DumpBuf,
    mem_writer::*,
    minidump_format::{MDRawDirectory, MDRawThreadName, MDStreamType},
};

impl MinidumpWriter {
    /// Writes the [`MDStreamType::ThreadNamesStream`] which is an array of
    /// [`MDRawThreadName`]
    pub(crate) fn write_thread_names(
        &mut self,
        buffer: &mut DumpBuf,
        dumper: &TaskDumper,
    ) -> Result<MDRawDirectory, super::super::WriterError> {
        let threads = dumper
            .read_threads()
            .map_err(|e| super::super::WriterError::TaskDumperError(e.to_string()))?;

        // Filter out handler thread
        let threads: Vec<_> = threads
            .iter()
            .filter(|&&tid| Some(tid) != self.handler_thread)
            .copied()
            .collect();

        let list_header = MemoryWriter::<u32>::alloc_with_val(buffer, threads.len() as u32)
            .map_err(|e| super::super::WriterError::MemoryWriterError(e.to_string()))?;

        let mut dirent = MDRawDirectory {
            stream_type: MDStreamType::ThreadNamesStream as u32,
            location: list_header.location(),
        };

        let mut names = MemoryArrayWriter::<MDRawThreadName>::alloc_array(buffer, threads.len())
            .map_err(|e| super::super::WriterError::MemoryWriterError(e.to_string()))?;
        dirent.location.data_size += names.location().data_size;

        for (i, &tid) in threads.iter().enumerate() {
            // It's unfortunate if we can't grab a thread name, but it's also
            // not a critical failure
            let name_loc = match Self::write_thread_name(buffer, dumper, tid) {
                Ok(loc) => loc,
                Err(err) => {
                    log::warn!("failed to write thread name for thread {tid}: {err}");
                    write_string_to_location(buffer, "")
                        .map_err(|e| super::super::WriterError::MemoryWriterError(e.to_string()))?
                }
            };

            let thread = MDRawThreadName {
                thread_id: tid,
                thread_name_rva: name_loc.rva.into(),
            };

            names
                .set_value_at(buffer, thread, i)
                .map_err(|e| super::super::WriterError::MemoryWriterError(e.to_string()))?;
        }

        Ok(dirent)
    }

    /// Attempts to retrieve and write the thread name, returning the thread names
    /// location if successful
    fn write_thread_name(
        buffer: &mut DumpBuf,
        dumper: &TaskDumper,
        tid: u32,
    ) -> Result<MDLocationDescriptor, super::super::WriterError> {
        // On iOS, we need to use thread_info with THREAD_EXTENDED_INFO flavor
        // However, this is not always available, so we fall back to empty name

        // For now, we'll just write empty names as thread naming on iOS
        // is more restricted than macOS
        // TODO: Investigate if we can use pthread_getname_np or similar

        Ok(write_string_to_location(buffer, "")
            .map_err(|e| super::super::WriterError::MemoryWriterError(e.to_string()))?)
    }
}
