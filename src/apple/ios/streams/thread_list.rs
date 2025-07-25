use crate::{
    apple::ios::{minidump_writer::MinidumpWriter, task_dumper::TaskDumper},
    mem_writer::{DumpBuf, MemoryArrayWriter, MemoryWriter},
    minidump_cpu::RawContextCPU,
    minidump_format::{
        MDLocationDescriptor, MDRawDirectory, MDRawThread, MDStreamType::ThreadListStream,
    },
};

type Result<T> = std::result::Result<T, super::StreamError>;

pub fn write(
    config: &mut MinidumpWriter,
    buffer: &mut DumpBuf,
    dumper: &TaskDumper,
) -> Result<(MDRawDirectory, Option<MDLocationDescriptor>)> {
    let threads = dumper.read_threads().unwrap_or_default();
    let num_threads = threads.len();

    let list_header = MemoryWriter::<u32>::alloc_with_val(buffer, num_threads as u32)
        .map_err(|e| super::StreamError::MemoryWriterError(e.to_string()))?;

    let mut dirent = MDRawDirectory {
        stream_type: ThreadListStream as u32,
        location: list_header.location(),
    };

    let mut thread_list = MemoryArrayWriter::<MDRawThread>::alloc_array(buffer, num_threads)
        .map_err(|e| super::StreamError::MemoryWriterError(e.to_string()))?;
    dirent.location.data_size += thread_list.location().data_size;

    let crashed_thread_id = config.crash_context.as_ref().map(|ctx| ctx.thread);
    let mut crashing_thread_context = None;

    for (idx, &tid) in threads.iter().enumerate() {
        let mut thread = MDRawThread {
            thread_id: tid,
            ..Default::default()
        };

        if Some(tid) == crashed_thread_id {
            if let Some(context) = &config.crash_context {
                // This is the crashing thread, use the context from the exception
                let mut cpu = RawContextCPU::default();
                context.fill_cpu_context(&mut cpu);
                let cpu_section = MemoryWriter::alloc_with_val(buffer, cpu)
                    .map_err(|e| super::StreamError::MemoryWriterError(e.to_string()))?;
                thread.thread_context = cpu_section.location();
                crashing_thread_context = Some(thread.thread_context);
            }
        } else {
            // For other threads, get the state from the dumper
            if let Ok(thread_state) = dumper.read_thread_state(tid) {
                let mut cpu = RawContextCPU::default();
                thread_state.fill_cpu_context(&mut cpu);
                let cpu_section = MemoryWriter::alloc_with_val(buffer, cpu)
                    .map_err(|e| super::StreamError::MemoryWriterError(e.to_string()))?;
                thread.thread_context = cpu_section.location();
            }
        }

        thread_list
            .set_value_at(buffer, thread, idx)
            .map_err(|e| super::StreamError::MemoryWriterError(e.to_string()))?;
    }

    Ok((dirent, crashing_thread_context))
}
