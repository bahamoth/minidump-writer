// IMPORTANT: iOS Signal Safety Considerations
// ============================================
// iOS only supports self-process dumps, which means this code may run inside
// signal handlers. The current implementation uses heap allocation (Vec/String)
// which is NOT signal-safe. This is a known architectural issue that needs
// to be addressed for production use in crash handlers.
//
// TODO: Implement signal-safe version using:
// - Pre-allocated fixed-size buffers
// - Direct write(2) system calls
// - No heap allocations or error propagation with allocations

use crate::{
    apple::{
        common::mach,
        ios::{
            minidump_writer::{MinidumpWriter, WriterError},
            task_dumper::TaskDumper,
        },
    },
    dir_section::DumpBuf,
    mem_writer::{write_string_to_location, MemoryArrayWriter, MemoryWriter},
    minidump_format::{MDLocationDescriptor, MDRawDirectory, MDRawModule, MDStreamType},
};

// We use dyld functions here which are marked deprecated in libc, but mach2 doesn't provide them yet
// TODO: Consider migrating to dyld_process_info_create when minimum iOS version is raised to 10.0+
// The _dyld_* functions have been deprecated since macOS 10.5 but are still available and widely used
// for compatibility with older iOS versions. The newer dyld_process_info_create API requires iOS 10.0+.
#[allow(deprecated)]
use libc::{_dyld_get_image_header, _dyld_get_image_name, _dyld_image_count};

struct ImageLoadInfo {
    /// The preferred load address of the TEXT segment
    vm_addr: u64,
    /// The size of the TEXT segment
    vm_size: u64,
    /// The difference between the image's preferred and actual load address
    slide: isize,
}

struct ImageDetails {
    uuid: [u8; 16],
    load_info: ImageLoadInfo,
    file_path: Option<String>,
    version: Option<u32>,
}

impl MinidumpWriter {
    /// Writes the module list stream for iOS
    pub(crate) fn write_module_list(
        &mut self,
        buffer: &mut DumpBuf,
        dumper: &TaskDumper,
    ) -> Result<MDRawDirectory, super::super::WriterError> {
        self.write_module_list_impl(buffer, dumper)
            .map_err(WriterError::from)
    }

    fn write_module_list_impl(
        &self,
        buffer: &mut DumpBuf,
        dumper: &TaskDumper,
    ) -> Result<MDRawDirectory, super::StreamError> {
        let modules = write_loaded_modules(buffer, dumper)?;

        let list_header = MemoryWriter::<u32>::alloc_with_val(buffer, modules.len() as u32)
            .map_err(|e| super::StreamError::MemoryWriterError(e.to_string()))?;

        let mut dirent = MDRawDirectory {
            stream_type: MDStreamType::ModuleListStream as u32,
            location: list_header.location(),
        };

        if !modules.is_empty() {
            let modules_section =
                MemoryArrayWriter::<MDRawModule>::alloc_from_iter(buffer, modules)
                    .map_err(|e| super::StreamError::MemoryWriterError(e.to_string()))?;
            dirent.location.data_size += modules_section.location().data_size;
        }

        Ok(dirent)
    }
}

fn write_loaded_modules(
    buf: &mut DumpBuf,
    dumper: &TaskDumper,
) -> Result<Vec<MDRawModule>, super::StreamError> {
    let (_all_images_info, mut images) = dumper
        .read_images()
        .map_err(|e| super::StreamError::MemoryWriterError(e.to_string()))?;

    // Sort by load address and remove duplicates (iOS can list the same image multiple times)
    images.sort();
    images.dedup();

    let mut modules = Vec::with_capacity(images.len());

    for image in images {
        // Read image details from Mach-O headers
        let details = match read_image_details(&image, dumper) {
            Ok(d) => d,
            Err(_) => continue, // Skip images we can't read
        };

        let mut module = MDRawModule {
            base_of_image: (details.load_info.vm_addr as isize + details.load_info.slide) as u64,
            size_of_image: details.load_info.vm_size as u32,
            checksum: 0,
            time_date_stamp: 0,
            module_name_rva: 0,
            version_info: details
                .version
                .map(|v| {
                    // macOS version format: 0xAABBCC00 where AA = major, BB = minor, CC = patch
                    let major = (v >> 16) & 0xffff;
                    let minor = (v >> 8) & 0xff;
                    let patch = v & 0xff;

                    minidump_common::format::VS_FIXEDFILEINFO {
                        signature: 0xfeef04bd,      // VS_FFI_SIGNATURE
                        struct_version: 0x00010000, // VS_FFI_STRUCVERSION
                        file_version_hi: (major << 16) | minor,
                        file_version_lo: patch << 16,
                        product_version_hi: (major << 16) | minor,
                        product_version_lo: patch << 16,
                        file_flags_mask: 0x3f, // VS_FFI_FILEFLAGSMASK
                        file_flags: 0,
                        file_os: 0x00040004,   // VOS_UNKNOWN
                        file_type: 0x00000001, // VFT_APP
                        file_subtype: 0,
                        file_date_hi: 0,
                        file_date_lo: 0,
                    }
                })
                .unwrap_or_default(),
            cv_record: MDLocationDescriptor {
                data_size: 0,
                rva: 0,
            },
            misc_record: MDLocationDescriptor {
                data_size: 0,
                rva: 0,
            },
            reserved0: [0; 2],
            reserved1: [0; 2],
        };

        // Write module path
        if let Some(ref path) = details.file_path {
            let path_location = write_string_to_location(buf, path)
                .map_err(|e| super::StreamError::MemoryWriterError(e.to_string()))?;
            module.module_name_rva = path_location.rva;
        }

        // Write CodeView record (UUID on macOS/iOS)
        // We need to write the CV record data manually instead of using a struct
        let cv_location = MDLocationDescriptor {
            data_size: 4 + 16 + 4, // cv_signature + uuid + age
            rva: buf.position() as u32,
        };

        // Write CV signature, UUID, and age
        // SAFETY WARNING: This code uses heap allocation (Vec) and is NOT signal-safe.
        // iOS requires self-process dumps which may run in signal handlers.
        // TODO: This needs to be rewritten to use pre-allocated buffers for signal safety.
        buf.write_all(&CV_SIGNATURE.to_le_bytes());
        buf.write_all(&details.uuid);
        buf.write_all(&0u32.to_le_bytes());

        module.cv_record = cv_location;

        modules.push(module);
    }

    Ok(modules)
}

fn read_image_details(
    image: &crate::apple::common::ImageInfo,
    dumper: &TaskDumper,
) -> Result<ImageDetails, crate::apple::common::TaskDumpError> {
    let mut load_info = None;
    let mut version = None;
    let mut uuid = None;

    // Read load commands from the image
    let load_commands = dumper.read_load_commands(image)?;

    for lc in load_commands.iter() {
        match lc {
            mach::LoadCommand::Segment(seg) if load_info.is_none() => {
                if &seg.segment_name[..7] == b"__TEXT\0" {
                    let slide = image.load_address as isize - seg.vm_addr as isize;

                    load_info = Some(ImageLoadInfo {
                        vm_addr: seg.vm_addr,
                        vm_size: seg.vm_size,
                        slide,
                    });
                }
            }
            mach::LoadCommand::Dylib(dylib) if version.is_none() => {
                version = Some(dylib.dylib.current_version);
            }
            mach::LoadCommand::Uuid(img_id) if uuid.is_none() => {
                uuid = Some(img_id.uuid);
            }
            _ => {}
        }

        if load_info.is_some() && version.is_some() && uuid.is_some() {
            break;
        }
    }

    let load_info = load_info.ok_or(crate::apple::common::TaskDumpError::MissingLoadCommand {
        name: "LC_SEGMENT_64",
        id: mach::LoadCommandKind::Segment,
    })?;
    let uuid = uuid.ok_or(crate::apple::common::TaskDumpError::MissingLoadCommand {
        name: "LC_UUID",
        id: mach::LoadCommandKind::Uuid,
    })?;

    // For iOS, we can use dyld API to get reliable file paths
    #[allow(deprecated)]
    let file_path = {
        let image_count = unsafe { _dyld_image_count() };
        let mut found_path = None;

        for i in 0..image_count {
            let header = unsafe { _dyld_get_image_header(i) };
            if header as u64 == image.load_address {
                let name_ptr = unsafe { _dyld_get_image_name(i) };
                if !name_ptr.is_null() {
                    let c_str = unsafe { std::ffi::CStr::from_ptr(name_ptr) };
                    found_path = c_str.to_str().ok().map(String::from);
                }
                break;
            }
        }
        found_path
    };

    Ok(ImageDetails {
        uuid,
        load_info,
        file_path,
        version,
    })
}

const CV_SIGNATURE: u32 = 0x53445352; // 'RSDS'
