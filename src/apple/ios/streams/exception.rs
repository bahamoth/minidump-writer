use crate::{
    apple::ios::minidump_writer::MinidumpWriter,
    apple::common::TaskDumper,
    dir_section::DumpBuf,
    mem_writer::MemoryWriter,
    minidump_format::{
        MDException, MDLocationDescriptor, MDRawDirectory, MDRawExceptionStream,
        MDStreamType::ExceptionStream,
    },
};

type Result<T> = std::result::Result<T, super::StreamError>;

impl MinidumpWriter {
    pub(crate) fn write_exception(
        &mut self,
        buffer: &mut DumpBuf,
        _dumper: &TaskDumper,
    ) -> std::result::Result<MDRawDirectory, super::super::WriterError> {
        self.write_exception_impl(buffer, self.crashing_thread_context)
            .map_err(super::super::WriterError::StreamError)
    }

    fn write_exception_impl(
        &self,
        buffer: &mut DumpBuf,
        thread_context: Option<MDLocationDescriptor>,
    ) -> Result<MDRawDirectory> {
        let exception_record = if let Some(context) = &self.crash_context {
            if let Some(exception) = &context.exception {
                MDException {
                    exception_code: exception.kind,
                    exception_flags: exception.code as u32, // Truncation is acceptable here
                    exception_address: exception.subcode.unwrap_or(0),
                    ..Default::default()
                }
            } else {
                MDException::default()
            }
        } else {
            MDException::default()
        };

        let crashed_thread_id = self.crash_context.as_ref().map_or(0, |ctx| ctx.thread);

        let stream = MDRawExceptionStream {
            thread_id: crashed_thread_id,
            exception_record,
            __align: 0,
            thread_context: thread_context.unwrap_or_default(),
        };

        let exc_section = MemoryWriter::alloc_with_val(buffer, stream)
            .map_err(|e| super::StreamError::MemoryWriterError(e.to_string()))?;

        Ok(MDRawDirectory {
            stream_type: ExceptionStream as u32,
            location: exc_section.location(),
        })
    }
}
