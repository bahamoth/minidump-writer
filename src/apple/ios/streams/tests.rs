#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::dir_section::DumpBuf;
    use crate::minidump_format::*;

    #[test]
    fn test_write_system_info() {
        let mut buffer = DumpBuf::new(0);

        // Write system info
        let result = system_info::write_system_info(&mut buffer);
        assert!(result.is_ok());

        let dirent = result.unwrap();
        assert_eq!(dirent.stream_type, MDStreamType::SystemInfoStream as u32);
        assert!(dirent.location.data_size > 0);
        assert_eq!(
            dirent.location.data_size as usize,
            std::mem::size_of::<MDRawSystemInfo>()
        );
    }

    #[test]
    fn test_system_info_contents() {
        let mut buffer = DumpBuf::new(0);

        // Write system info
        let result = system_info::write_system_info(&mut buffer);
        assert!(result.is_ok());

        // Read back the system info
        let bytes = buffer.as_bytes();
        let dirent = result.unwrap();
        let offset = dirent.location.rva as usize;

        // SAFETY: We know the buffer contains valid MDRawSystemInfo at this offset
        let sys_info = unsafe {
            let ptr = bytes.as_ptr().add(offset) as *const MDRawSystemInfo;
            &*ptr
        };

        // Verify iOS platform ID
        assert_eq!(sys_info.platform_id, 0x8000); // iOS platform

        // Verify processor architecture
        assert_eq!(
            sys_info.processor_architecture,
            MDCPUArchitecture::PROCESSOR_ARCHITECTURE_ARM64_OLD as u16
        );

        // Verify processor count
        assert!(sys_info.number_of_processors >= 2); // iOS devices have at least 2 cores

        // Verify OS version
        assert!(sys_info.major_version >= 12); // iOS 12+
    }

    #[test]
    fn test_minidump_writer_with_system_info() {
        use crate::apple::ios::MinidumpWriter;
        use std::io::Cursor;

        let mut writer = MinidumpWriter::new();
        let mut cursor = Cursor::new(Vec::new());

        // Dump to cursor
        let result = writer.dump(&mut cursor);
        assert!(result.is_ok());

        let bytes = result.unwrap();
        assert!(!bytes.is_empty());

        // Verify header
        let header = unsafe {
            let ptr = bytes.as_ptr() as *const MDRawHeader;
            &*ptr
        };

        assert_eq!(header.signature, MINIDUMP_SIGNATURE);
        assert_eq!(header.version, MINIDUMP_VERSION);
        assert!(header.stream_count >= 1); // At least system info stream
    }
}
