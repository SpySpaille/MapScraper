use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct GmaEntry {
    pub file_name: String,
    pub size: u64,
    pub offset: u64,
}

pub struct GmaArchive {
    pub label: String,
    pub path: PathBuf,
    pub name: String,
    pub author: String,
    pub entries: HashMap<String, GmaEntry>,
    file_data_start: u64,
}

impl GmaArchive {
    pub fn open<P: AsRef<Path>>(path: P, label: &str) -> io::Result<Self> {
        let path_buf = path.as_ref().to_path_buf();
        let mut file = File::open(&path_buf)?;

        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)?;
        if &magic != b"GMAD" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid GMA header magic",
            ));
        }

        let mut version = [0u8; 1];
        file.read_exact(&mut version)?;
        let format_version = version[0];

        // SteamID (8 bytes) + Timestamp (8 bytes)
        let mut dummy16 = [0u8; 16];
        file.read_exact(&mut dummy16)?;

        // Required content (if version > 1)
        if format_version > 1 {
            read_null_string(&mut file)?;
        }

        let addon_name = read_null_string(&mut file)?;
        let _addon_desc = read_null_string(&mut file)?;
        let addon_author = read_null_string(&mut file)?;

        // Addon version (4 bytes)
        let mut dummy4 = [0u8; 4];
        file.read_exact(&mut dummy4)?;

        // Read file entries
        let mut entry_list = Vec::new();
        loop {
            let mut entry_num_bytes = [0u8; 4];
            file.read_exact(&mut entry_num_bytes)?;
            let entry_num = u32::from_le_bytes(entry_num_bytes);
            if entry_num == 0 {
                break;
            }

            let entry_path = read_null_string(&mut file)?;

            let mut size_bytes = [0u8; 8];
            file.read_exact(&mut size_bytes)?;
            let size = u64::from_le_bytes(size_bytes);

            // CRC (4 bytes)
            let mut crc_bytes = [0u8; 4];
            file.read_exact(&mut crc_bytes)?;

            entry_list.push((entry_path, size));
        }

        let file_data_start = file.stream_position()?;

        let mut entries = HashMap::new();
        let mut current_offset = 0u64;

        for (filename, size) in entry_list {
            let norm = filename.replace('\\', "/").to_lowercase();
            entries.insert(
                norm.clone(),
                GmaEntry {
                    file_name: filename,
                    size,
                    offset: current_offset,
                },
            );
            current_offset += size;
        }

        Ok(Self {
            label: label.to_string(),
            path: path_buf,
            name: addon_name,
            author: addon_author,
            entries,
            file_data_start,
        })
    }

    pub fn has_file(&self, rel_path: &str) -> bool {
        let norm = rel_path.replace('\\', "/").to_lowercase();
        self.entries.contains_key(&norm)
    }

    pub fn read_file(&self, rel_path: &str) -> io::Result<Vec<u8>> {
        let norm = rel_path.replace('\\', "/").to_lowercase();
        let entry = self.entries.get(&norm).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("File not found in GMA: {}", rel_path),
            )
        })?;

        let mut file = File::open(&self.path)?;
        file.seek(SeekFrom::Start(self.file_data_start + entry.offset))?;
        let mut buf = vec![0u8; entry.size as usize];
        file.read_exact(&mut buf)?;
        Ok(buf)
    }
}

fn read_null_string<R: Read>(reader: &mut R) -> io::Result<String> {
    let mut bytes = Vec::new();
    let mut buf = [0u8; 1];
    loop {
        reader.read_exact(&mut buf)?;
        if buf[0] == 0 {
            break;
        }
        bytes.push(buf[0]);
    }
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_gma_parser_synthetic() {
        let temp_dir = std::env::temp_dir();
        let gma_path = temp_dir.join("test_sample.gma");

        let mut f = File::create(&gma_path).unwrap();
        // Magic
        f.write_all(b"GMAD").unwrap();
        // Format version
        f.write_all(&[3u8]).unwrap();
        // SteamID (8) + Timestamp (8)
        f.write_all(&[0u8; 16]).unwrap();
        // Required content (empty string)
        f.write_all(b"\0").unwrap();
        // Name
        f.write_all(b"Test Addon\0").unwrap();
        // Desc
        f.write_all(b"Test Description\0").unwrap();
        // Author
        f.write_all(b"Author Name\0").unwrap();
        // Addon version
        f.write_all(&1i32.to_le_bytes()).unwrap();

        // Entry 1
        f.write_all(&1u32.to_le_bytes()).unwrap();
        f.write_all(b"materials/custom/sample.vmt\0").unwrap();
        let data = b"VertexLitGeneric\n{\n\t\"$basetexture\" \"custom/sample\"\n}";
        f.write_all(&(data.len() as u64).to_le_bytes()).unwrap();
        f.write_all(&0u32.to_le_bytes()).unwrap(); // crc

        // End of entries
        f.write_all(&0u32.to_le_bytes()).unwrap();

        // Data payload
        f.write_all(data).unwrap();
        drop(f);

        let gma = GmaArchive::open(&gma_path, "Test Addon").unwrap();
        assert_eq!(gma.name, "Test Addon");
        assert!(gma.has_file("materials/custom/sample.vmt"));
        let bytes = gma.read_file("materials/custom/sample.vmt").unwrap();
        assert_eq!(bytes, data);

        let _ = std::fs::remove_file(&gma_path);
    }
}
