use crate::{
    apple::ios::{minidump_writer::MinidumpWriter, task_dumper::TaskDumper},
    dir_section::DumpBuf,
    mem_writer::*,
    minidump_format::{MDLocationDescriptor, MDRawDirectory, MDRawThreadName, MDStreamType},
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
        _buffer: &mut DumpBuf,
        _dumper: &TaskDumper,
        _tid: u32,
    ) -> Result<MDLocationDescriptor, super::super::WriterError> {
        // On iOS retrieving thread names reliably is difficult – the
        // [`THREAD_EXTENDED_INFO`](https://developer.apple.com/documentation/kernel/thread_extended_info_data_t)
        // flavor is not available to sandboxed processes and `pthread_getname_np` is
        // restricted when the target thread lives in another task.  The Breakpad
        // minidump format explicitly allows `thread_name_rva` to be `0` to
        // indicate that a thread does not have an associated name.  Using `0`
        // is more accurate than writing an empty string because it allows
        // consumers to distinguish between “the writer could not determine the
        // name” and “the name is an empty string”.  Down-stream code such as
        // Mozilla’s `minidump-stackwalk` already relies on this semantics.

        Ok(MDLocationDescriptor {
            rva: 0,
            data_size: 0,
        })
    }
}
