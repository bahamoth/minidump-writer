use super::*;
use crate::apple::common::streams::memory_list::MemoryListStream;

impl MemoryListStream for MinidumpWriter {
    fn memory_blocks_mut(&mut self) -> &mut Vec<MDMemoryDescriptor> {
        &mut self.memory_blocks
    }

    fn crash_context(&self) -> Option<&crate::apple::common::CrashContext> {
        self.crash_context.as_ref()
    }
}

impl MinidumpWriter {
    /// Writes the memory list stream containing memory blocks collected from other streams
    pub(crate) fn write_memory_list(
        &mut self,
        buffer: &mut DumpBuf,
        dumper: &TaskDumper,
    ) -> Result<MDRawDirectory, WriterError> {
        MemoryListStream::write_memory_list(self, buffer, dumper).map_err(WriterError::from)
    }
}
