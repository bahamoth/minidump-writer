use super::*;
use crate::apple::common::streams::memory_list::MemoryListStream;

impl MemoryListStream for MinidumpWriter {
    fn memory_blocks_mut(&mut self) -> &mut Vec<MDMemoryDescriptor> {
        &mut self.memory_blocks
    }

    fn crash_context(&self) -> Option<&crate::apple::common::types::CrashContext> {
        // On iOS, CrashContext is a type alias for IosCrashContext
        cfg_if::cfg_if! {
            if #[cfg(target_os = "ios")] {
                self.crash_context.as_ref()
            } else {
                // When testing iOS on macOS, we need to handle the type mismatch
                None
            }
        }
    }
}

impl MinidumpWriter {
    /// Writes the memory list stream containing memory blocks collected from other streams
    pub(crate) fn write_memory_list(
        &mut self,
        buffer: &mut DumpBuf,
        dumper: &TaskDumper,
    ) -> Result<MDRawDirectory, WriterError> {
        use crate::apple::common::streams::memory_list::StreamError;

        MemoryListStream::write_memory_list(self, buffer, dumper).map_err(|e| match e {
            StreamError::MemoryWriter(e) => WriterError::MemoryWriterError(e),
            StreamError::TaskDump(e) => WriterError::TaskDumperError(e),
        })
    }
}
