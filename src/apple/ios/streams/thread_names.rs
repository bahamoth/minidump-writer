use super::*;
use crate::mem_writer::*;

impl MinidumpWriter {
    /// Writes the [`MDStreamType::ThreadNamesStream`] with empty names
    ///
    /// iOS cannot retrieve thread names due to sandbox restrictions.
    /// All thread_name_rva values will be 0, indicating "name unavailable".
    pub(crate) fn write_thread_names(
        &mut self,
        buffer: &mut DumpBuf,
        dumper: &TaskDumper,
    ) -> Result<MDRawDirectory, WriterError> {
        let threads = dumper
            .read_threads()
            .map_err(|e| WriterError::TaskDumperError(e.to_string()))?;

        // Filter out handler thread
        let threads: Vec<_> = threads
            .iter()
            .filter(|&&tid| Some(tid) != self.handler_thread)
            .copied()
            .collect();

        let list_header = MemoryWriter::<u32>::alloc_with_val(buffer, threads.len() as u32)
            .map_err(|e| WriterError::MemoryWriterError(e.to_string()))?;

        let mut dirent = MDRawDirectory {
            stream_type: MDStreamType::ThreadNamesStream as u32,
            location: list_header.location(),
        };

        let mut names = MemoryArrayWriter::<MDRawThreadName>::alloc_array(buffer, threads.len())
            .map_err(|e| WriterError::MemoryWriterError(e.to_string()))?;
        dirent.location.data_size += names.location().data_size;

        // Write all thread IDs with name_rva = 0 (name unavailable)
        for (i, &tid) in threads.iter().enumerate() {
            let thread = MDRawThreadName {
                thread_id: tid,
                thread_name_rva: 0, // 0 means "name unavailable" in Breakpad format
            };

            names
                .set_value_at(buffer, thread, i)
                .map_err(|e| WriterError::MemoryWriterError(e.to_string()))?;
        }

        Ok(dirent)
    }
}
