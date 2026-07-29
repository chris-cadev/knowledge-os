use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum DetectedFormat {
    Docx,
    Xlsx,
    Pptx,
    Pdf,
    Zip,
    Unknown,
}

/// Detect format by reading magic bytes from a file.
pub fn detect_format(path: &Path) -> Result<DetectedFormat, std::io::Error> {
    let mut buf = [0u8; 8];
    let mut file = std::fs::File::open(path)?;
    use std::io::Read;
    let n = file.read(&mut buf)?;
    if n < 4 {
        return Ok(DetectedFormat::Unknown);
    }
    Ok(match &buf[..4] {
        [0x50, 0x4B, 0x03, 0x04] => {
            if buf.len() >= 6 {
                let extra = u16::from_le_bytes([buf[4], buf[5]]);
                if extra != 0 {
                    if n >= 8 {
                        let extra2 = u16::from_le_bytes([buf[6], buf[7]]);
                        match extra2 {
                            0x0061 => DetectedFormat::Docx,
                            0x0062 => DetectedFormat::Xlsx,
                            0x0063 => DetectedFormat::Pptx,
                            _ => DetectedFormat::Zip,
                        }
                    } else {
                        DetectedFormat::Zip
                    }
                } else {
                    DetectedFormat::Zip
                }
            } else {
                DetectedFormat::Zip
            }
        }
        [0x25, 0x50, 0x44, 0x46] => DetectedFormat::Pdf,
        _ => DetectedFormat::Unknown,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_file_with_bytes(bytes: &[u8]) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(bytes).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_magic_bytes_detects_pdf() {
        let file = create_file_with_bytes(b"%PDF-1.4\n...");
        assert_eq!(detect_format(file.path()).unwrap(), DetectedFormat::Pdf);
    }

    #[test]
    fn test_magic_bytes_unknown() {
        let file = create_file_with_bytes(b"not a known format");
        assert_eq!(detect_format(file.path()).unwrap(), DetectedFormat::Unknown);
    }

    #[test]
    fn test_magic_bytes_empty_file() {
        let file = create_file_with_bytes(b"");
        assert_eq!(detect_format(file.path()).unwrap(), DetectedFormat::Unknown);
    }
}
