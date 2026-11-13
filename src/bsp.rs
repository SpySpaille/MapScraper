use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct BspEntry {
    pub offset: u64,
    pub comp_size: u64,
    pub uncomp_size: u64,
    pub method: u16,
}

pub struct BspArchive {
    pub path: PathBuf,
    pub label: String,
    pub entries: HashMap<String, BspEntry>,
}

impl BspArchive {
    pub fn open(path: &Path) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let mut header = [0u8; 1036];
        file.read_exact(&mut header)?;

        if &header[0..4] != b"VBSP" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Not a valid VBSP file",
            ));
        }

        let lump_idx = 40; // LUMP_PAKFILE
        let lump_pos = 8 + lump_idx * 16;
        let invalid_pak = || io::Error::new(io::ErrorKind::InvalidData, "Invalid BSP pakfile");
        let lump_offset =
            u32::from_le_bytes(header[lump_pos..lump_pos + 4].try_into().unwrap()) as u64;
        let lump_len =
            u32::from_le_bytes(header[lump_pos + 4..lump_pos + 8].try_into().unwrap()) as u64;
        let lump_end = lump_offset + lump_len;

        let mut entries = HashMap::new();
        if lump_len != 0 {
            if lump_len < 22 || lump_end > file.metadata()?.len() {
                return Err(invalid_pak());
            }
            // Read only the ZIP directory and local headers; pakfiles can be hundreds of MB.
            let tail_len = lump_len.min(22 + u16::MAX as u64);
            let mut tail = vec![0; tail_len as usize];
            file.seek(SeekFrom::Start(lump_end - tail_len))?;
            file.read_exact(&mut tail)?;
            let u16_at = |bytes: &[u8], i| u16::from_le_bytes(bytes[i..i + 2].try_into().unwrap());
            let u32_at = |bytes: &[u8], i| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
            let end = (0..=tail.len() - 22)
                .rev()
                .find(|&i| {
                    &tail[i..i + 4] == b"PK\x05\x06"
                        && (i + 22 + u16_at(&tail, i + 20) as usize <= tail.len())
                })
                .ok_or_else(invalid_pak)?;
            let directory_end = lump_end - tail_len + end as u64;
            let mut cursor = lump_offset + u32_at(&tail, end + 16) as u64;

            for _ in 0..u16_at(&tail, end + 10) {
                let mut central = [0; 46];
                if cursor + 46 > directory_end {
                    return Err(invalid_pak());
                }
                file.seek(SeekFrom::Start(cursor))?;
                file.read_exact(&mut central)?;
                if &central[..4] != b"PK\x01\x02" {
                    return Err(invalid_pak());
                }
                let name_len = u16_at(&central, 28) as usize;
                cursor += 46
                    + name_len as u64
                    + u16_at(&central, 30) as u64
                    + u16_at(&central, 32) as u64;
                if cursor > directory_end {
                    return Err(invalid_pak());
                }
                let mut name = vec![0; name_len];
                file.read_exact(&mut name)?;
                let norm_name = String::from_utf8_lossy(&name)
                    .replace('\\', "/")
                    .trim_start_matches('/')
                    .to_lowercase();

                let local_offset = lump_offset + u32_at(&central, 42) as u64;
                let mut local = [0; 30];
                if local_offset + 30 > directory_end {
                    return Err(invalid_pak());
                }
                file.seek(SeekFrom::Start(local_offset))?;
                file.read_exact(&mut local)?;
                if &local[..4] != b"PK\x03\x04" {
                    return Err(invalid_pak());
                }
                let offset =
                    local_offset + 30 + u16_at(&local, 26) as u64 + u16_at(&local, 28) as u64;
                let method = u16_at(&central, 10);
                let comp_size = u32_at(&central, 20) as u64;
                let uncomp_size = u32_at(&central, 24) as u64;
                if offset + comp_size > directory_end || (method == 0 && comp_size != uncomp_size) {
                    return Err(invalid_pak());
                }
                entries.insert(
                    norm_name,
                    BspEntry {
                        offset,
                        comp_size,
                        uncomp_size,
                        method,
                    },
                );
            }
        }

        let label = format!(
            "BSP ({})",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        Ok(Self {
            path: path.to_path_buf(),
            label,
            entries,
        })
    }

    pub fn has_file(&self, rel_path: &str) -> bool {
        let norm = rel_path
            .replace('\\', "/")
            .trim_start_matches('/')
            .to_lowercase();
        self.entries.contains_key(&norm)
    }

    pub fn read_file(&self, rel_path: &str) -> io::Result<Vec<u8>> {
        let norm = rel_path
            .replace('\\', "/")
            .trim_start_matches('/')
            .to_lowercase();
        let entry = self.entries.get(&norm).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("File not found in BSP: {}", rel_path),
            )
        })?;

        if entry.method == 0 {
            let mut file = File::open(&self.path)?;
            file.seek(SeekFrom::Start(entry.offset))?;
            let mut buf = vec![0u8; entry.uncomp_size as usize];
            file.read_exact(&mut buf)?;
            Ok(buf)
        } else {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!(
                    "Compression method {} in BSP pakfile not supported",
                    entry.method
                ),
            ))
        }
    }

    pub fn extract_file(&self, rel_path: &str, dest: &Path) -> io::Result<u64> {
        let bytes = self.read_file(rel_path)?;
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(dest, &bytes)?;
        Ok(bytes.len() as u64)
    }
}
