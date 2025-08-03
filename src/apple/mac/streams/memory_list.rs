use super::*;
use crate::apple::common::streams::memory_list::MemoryListStream;

impl MemoryListStream for MinidumpWriter {
    fn memory_blocks_mut(&mut self) -> &mut Vec<MDMemoryDescriptor> {
        &mut self.memory_blocks
    }

    fn crash_context(&self) -> Option<&crate::apple::common::types::CrashContext> {
        self.crash_context.as_ref()
    }
}

impl MinidumpWriter {
    /// Writes the [`MDStreamType::MemoryListStream`]. The memory blocks that are
    /// written into this stream are the raw thread contexts that were retrieved
    /// and added by [`Self::write_thread_list`]
    pub(crate) fn write_memory_list(
        &mut self,
        buffer: &mut DumpBuf,
        dumper: &TaskDumper,
    ) -> Result<MDRawDirectory, WriterError> {
        use crate::apple::common::streams::memory_list::StreamError;
        
        MemoryListStream::write_memory_list(self, buffer, dumper)
            .map_err(|e| match e {
                StreamError::MemoryWriter(e) => WriterError::MemoryWriterError(e),
                StreamError::TaskDump(e) => WriterError::TaskDumpError(e),
            })
    }
}
