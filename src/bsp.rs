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
        let lump_offset =
            i32::from_le_bytes(header[lump_pos..lump_pos + 4].try_into().unwrap()) as u64;
        let lump_len =
            i32::from_le_bytes(header[lump_pos + 4..lump_pos + 8].try_into().unwrap()) as u64;

        let mut entries = HashMap::new();
        if lump_len > 22 {
            let mut lump_bytes = vec![0u8; lump_len as usize];
            file.seek(SeekFrom::Start(lump_offset))?;
            file.read_exact(&mut lump_bytes)?;

            // Find End of Central Directory record (0x06054b50)
            let mut eocd_pos = None;
            for i in (0..lump_bytes.len().saturating_sub(22)).rev() {
                if &lump_bytes[i..i + 4] == b"PK\x05\x06" {
                    eocd_pos = Some(i);
                    break;
                }
            }

            if let Some(pos) = eocd_pos {
                let cd_offset = u32::from_le_bytes(
                    lump_bytes[pos + 16..pos + 20].try_into().unwrap(),
                ) as usize;
                let cd_entries = u16::from_le_bytes(
                    lump_bytes[pos + 10..pos + 12].try_into().unwrap(),
                ) as usize;

                let mut cur = cd_offset;
                for _ in 0..cd_entries {
                    if cur + 46 > lump_bytes.len() || &lump_bytes[cur..cur + 4] != b"PK\x01\x02" {
                        break;
                    }
                    let method =
                        u16::from_le_bytes(lump_bytes[cur + 10..cur + 12].try_into().unwrap());
                    let comp_size =
                        u32::from_le_bytes(lump_bytes[cur + 20..cur + 24].try_into().unwrap())
                            as u64;
                    let uncomp_size =
                        u32::from_le_bytes(lump_bytes[cur + 24..cur + 28].try_into().unwrap())
                            as u64;
                    let fname_len =
                        u16::from_le_bytes(lump_bytes[cur + 28..cur + 30].try_into().unwrap())
                            as usize;
                    let extra_len =
                        u16::from_le_bytes(lump_bytes[cur + 30..cur + 32].try_into().unwrap())
                            as usize;
                    let comment_len =
                        u16::from_le_bytes(lump_bytes[cur + 32..cur + 34].try_into().unwrap())
                            as usize;
                    let local_hdr_offset =
                        u32::from_le_bytes(lump_bytes[cur + 42..cur + 46].try_into().unwrap())
                            as usize;

                    let raw_name =
                        String::from_utf8_lossy(&lump_bytes[cur + 46..cur + 46 + fname_len]);
                    let norm_name = raw_name
                        .replace('\\', "/")
                        .trim_start_matches('/')
                        .to_lowercase();

                    if local_hdr_offset + 30 <= lump_bytes.len()
                        && &lump_bytes[local_hdr_offset..local_hdr_offset + 4] == b"PK\x03\x04"
                    {
                        let l_fname_len = u16::from_le_bytes(
                            lump_bytes[local_hdr_offset + 26..local_hdr_offset + 28]
                                .try_into()
                                .unwrap(),
                        ) as usize;
                        let l_extra_len = u16::from_le_bytes(
                            lump_bytes[local_hdr_offset + 28..local_hdr_offset + 30]
                                .try_into()
                                .unwrap(),
                        ) as usize;
                        let data_start = lump_offset
                            + local_hdr_offset as u64
                            + 30
                            + l_fname_len as u64
                            + l_extra_len as u64;

                        entries.insert(
                            norm_name,
                            BspEntry {
                                offset: data_start,
                                comp_size,
                                uncomp_size,
                                method,
                            },
                        );
                    }

                    cur += 46 + fname_len + extra_len + comment_len;
                }
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
