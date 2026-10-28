use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use vpk::VPK;

pub struct VpkArchive {
    pub label: String,
    pub path: PathBuf,
    pub vpk: VPK,
    entry_index: HashMap<String, String>,
}

impl VpkArchive {
    pub fn open<P: AsRef<Path>>(path: P, label: &str) -> Option<Self> {
        let p = path.as_ref().to_path_buf();
        if !p.exists() {
            return None;
        }

        let path_str = p.to_str()?;
        let archive = vpk::from_path(path_str).ok()?;
        let mut entry_index = HashMap::new();

        for full_path in archive.tree.keys() {
            let norm = full_path.replace('\\', "/").to_lowercase();
            entry_index.insert(norm, full_path.clone());
        }

        Some(Self {
            label: label.to_string(),
            path: p,
            vpk: archive,
            entry_index,
        })
    }

    pub fn has_file(&self, rel_path: &str) -> bool {
        let norm = rel_path.replace('\\', "/").to_lowercase();
        self.entry_index.contains_key(&norm)
    }

    pub fn read_file(&self, rel_path: &str) -> io::Result<Vec<u8>> {
        let norm = rel_path.replace('\\', "/").to_lowercase();
        let actual_key = self.entry_index.get(&norm).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("File not found in VPK: {}", rel_path),
            )
        })?;

        let entry = self.vpk.tree.get(actual_key).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Entry missing in VPK tree: {}", rel_path),
            )
        })?;

        let data = entry
            .get()
            .map_err(|e| io::Error::other(format!("Failed reading VPK entry: {:?}", e)))?;

        Ok(data.into_owned())
    }

    pub fn extract_file<P: AsRef<Path>>(&self, rel_path: &str, dest: P) -> io::Result<u64> {
        let bytes = self.read_file(rel_path)?;
        let dest_ref = dest.as_ref();
        if let Some(parent) = dest_ref.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(dest_ref, &bytes)?;
        Ok(bytes.len() as u64)
    }
    pub fn entries(&self) -> impl Iterator<Item = &String> {
        self.entry_index.keys()
    }
}
