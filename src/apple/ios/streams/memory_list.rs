use crate::{
    apple::ios::{minidump_writer::MinidumpWriter, task_dumper::TaskDumper},
    mem_writer::{DumpBuf, MemoryArrayWriter, MemoryWriter},
    minidump_format::{
        MDLocationDescriptor, MDMemoryDescriptor, MDRawDirectory, MDStreamType::MemoryListStream,
    },
};

type Result<T> = std::result::Result<T, super::StreamError>;

/// Writes the memory list stream containing memory blocks collected from other streams
pub fn write(
    config: &mut MinidumpWriter,
    buffer: &mut DumpBuf,
    dumper: &TaskDumper,
) -> Result<MDRawDirectory> {
    // Include some memory around the instruction pointer if the crash was
    // due to an exception
    if let Some(cc) = &config.crash_context {
        if cc.exception.is_some() {
            const IP_MEM_SIZE: u64 = 256;

            let get_ip_block = |tid| -> Option<std::ops::Range<u64>> {
                let thread_state = dumper.read_thread_state(tid).ok()?;
                let ip = thread_state.pc();

                // Bound it to the upper and lower bounds of the region
                // it's contained within. If it's not in a known memory region,
                // don't bother trying to write it.
                let region = dumper.get_vm_region(ip).ok()?;

                if ip < region.range.start || ip > region.range.end {
                    return None;
                }

                // Try to get IP_MEM_SIZE / 2 bytes before and after the IP, but
                // settle for whatever's available.
                let start = std::cmp::max(region.range.start, ip - IP_MEM_SIZE / 2);
                let end = std::cmp::min(ip + IP_MEM_SIZE / 2, region.range.end);

                Some(start..end)
            };

            if let Some(ip_range) = get_ip_block(cc.thread) {
                let size = ip_range.end - ip_range.start;

                // Try to read the memory around the instruction pointer
                // iOS sandbox restrictions may prevent access to some regions
                if let Ok(stack_buffer) =
                    dumper.read_task_memory::<u8>(ip_range.start, size as usize)
                {
                    let ip_location = MDLocationDescriptor {
                        data_size: stack_buffer.len() as u32,
                        rva: buffer.position() as u32,
                    };
                    buffer.write_all(&stack_buffer)?;

                    config.memory_blocks.push(MDMemoryDescriptor {
                        start_of_memory_range: ip_range.start,
                        memory: ip_location,
                    });
                }
            }
        }
    }

    // Write the memory list header (count of memory blocks)
    let list_header =
        MemoryWriter::<u32>::alloc_with_val(buffer, config.memory_blocks.len() as u32)
            .map_err(|e| super::StreamError::MemoryWriterError(e.to_string()))?;

    let mut dirent = MDRawDirectory {
        stream_type: MemoryListStream as u32,
        location: list_header.location(),
    };

    // Write the array of memory descriptors
    let block_list =
        MemoryArrayWriter::<MDMemoryDescriptor>::alloc_from_array(buffer, &config.memory_blocks)
            .map_err(|e| super::StreamError::MemoryWriterError(e.to_string()))?;

    dirent.location.data_size += block_list.location().data_size;
    Ok(dirent)
}
