use crate::{
    apple::ios::{minidump_writer::MinidumpWriter, task_dumper::TaskDumper},
    mem_writer::{DumpBuf, MemoryArrayWriter, MemoryWriter},
    minidump_cpu::RawContextCPU,
    minidump_format::{
        MDLocationDescriptor, MDMemoryDescriptor, MDRawDirectory, MDRawThread,
        MDStreamType::ThreadListStream,
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

        // Handle thread state and context
        if Some(tid) == crashed_thread_id {
            if let Some(context) = &config.crash_context {
                // This is the crashing thread, use the context from the exception
                let mut cpu = RawContextCPU::default();
                context.fill_cpu_context(&mut cpu);
                let cpu_section = MemoryWriter::alloc_with_val(buffer, cpu)
                    .map_err(|e| super::StreamError::MemoryWriterError(e.to_string()))?;
                thread.thread_context = cpu_section.location();
                crashing_thread_context = Some(thread.thread_context);

                // Get stack pointer from crash context
                let sp = context.thread_state.sp();
                write_stack_from_start_address(sp, &mut thread, buffer, dumper, config)?;
            }
        } else {
            // For other threads, get the state from the dumper
            if let Ok(thread_state) = dumper.read_thread_state(tid) {
                let mut cpu = RawContextCPU::default();
                thread_state.fill_cpu_context(&mut cpu);
                let cpu_section = MemoryWriter::alloc_with_val(buffer, cpu)
                    .map_err(|e| super::StreamError::MemoryWriterError(e.to_string()))?;
                thread.thread_context = cpu_section.location();

                // Get stack pointer and write stack memory
                let sp = thread_state.sp();
                write_stack_from_start_address(sp, &mut thread, buffer, dumper, config)?;
            }
        }

        // Try to get thread priority and suspend count
        if let Ok(basic_info) =
            dumper.thread_info::<mach2::thread_basic_info::thread_basic_info_t>(tid)
        {
            thread.suspend_count = basic_info.suspend_count as u32;
            // Priority is a complex calculation on macOS/iOS, using policy as proxy
            thread.priority = basic_info.policy as u32;
        }

        thread_list
            .set_value_at(buffer, thread, idx)
            .map_err(|e| super::StreamError::MemoryWriterError(e.to_string()))?;
    }

    Ok((dirent, crashing_thread_context))
}

/// Write stack memory for a thread
fn write_stack_from_start_address(
    start: u64,
    thread: &mut MDRawThread,
    buffer: &mut DumpBuf,
    dumper: &TaskDumper,
    config: &mut MinidumpWriter,
) -> Result<()> {
    thread.stack.start_of_memory_range = start;
    thread.stack.memory.data_size = 0;
    thread.stack.memory.rva = buffer.position() as u32;

    let stack_size = calculate_stack_size(start, dumper);

    // In some situations the stack address for the thread can come back 0.
    // In these cases we skip over the threads in question and stuff the
    // stack with a clearly borked value.
    //
    // In other cases, notably a stack overflow, we might fail to read the
    // stack eg. InvalidAddress in which case we use a different borked
    // value to indicate the different failure
    let stack_location = if stack_size != 0 {
        dumper
            .read_task_memory::<u8>(start, stack_size)
            .map(|stack_buffer| {
                let stack_location = MDLocationDescriptor {
                    data_size: stack_buffer.len() as u32,
                    rva: buffer.position() as u32,
                };
                buffer.write_all(&stack_buffer)?;
                stack_location
            })
            .ok()
    } else {
        None
    };

    thread.stack.memory = stack_location.unwrap_or_else(|| {
        let borked = if stack_size == 0 {
            0xdeadbeef
        } else {
            0xdeaddead
        };

        thread.stack.start_of_memory_range = borked;

        let stack_location = MDLocationDescriptor {
            data_size: 16,
            rva: buffer.position() as u32,
        };
        buffer.write_all(&borked.to_ne_bytes());
        buffer.write_all(&borked.to_ne_bytes());
        stack_location
    });

    // Add the stack memory as a raw block of memory, this is written to
    // the minidump as part of the memory list stream
    config.memory_blocks.push(thread.stack);
    Ok(())
}

/// Calculate the size of the stack for the given start address
fn calculate_stack_size(start_address: u64, dumper: &TaskDumper) -> usize {
    if start_address == 0 {
        return 0;
    }

    let mut region = if let Ok(region) = dumper.get_vm_region(start_address) {
        region
    } else {
        return 0;
    };

    // Failure or stack corruption, since vm_region had to go
    // higher in the process address space to find a valid region.
    if start_address < region.range.start {
        return 0;
    }

    let root_range_start = region.range.start;
    let mut stack_size = region.range.end - region.range.start;

    // If the user tag is VM_MEMORY_STACK, look for more readable regions with
    // the same tag placed immediately above the computed stack region. Under
    // some circumstances, the stack for thread 0 winds up broken up into
    // multiple distinct abutting regions. This can happen for several reasons,
    // including user code that calls setrlimit(RLIMIT_STACK, ...) or changes
    // the access on stack pages by calling mprotect.
    if region.info.user_tag == mach2::vm_statistics::VM_MEMORY_STACK {
        loop {
            let proposed_next_region_base = region.range.end;

            region = if let Ok(reg) = dumper.get_vm_region(region.range.end) {
                reg
            } else {
                break;
            };

            if region.range.start != proposed_next_region_base
                || region.info.user_tag != mach2::vm_statistics::VM_MEMORY_STACK
                || (region.info.protection & mach2::vm_prot::VM_PROT_READ) == 0
            {
                break;
            }

            stack_size += region.range.end - region.range.start;
        }
    }

    (root_range_start + stack_size - start_address) as usize
}
