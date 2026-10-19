use crate::paths::VirtualAssetPath;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetKind {
    Material,
    Texture,
    Model,
    Sound,
    Particle,
    Script,
    Instance,
    Auxiliary,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetStatus {
    Discovered,
    Extracted,
    Missing,
    Failed(String),
    SkippedStock,
    AlreadyPresent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRecord {
    pub virtual_path: VirtualAssetPath,
    pub kind: AssetKind,
    pub source_path: Option<PathBuf>,
    pub dest_path: Option<PathBuf>,
    pub status: AssetStatus,
    pub size_bytes: u64,
    pub reason: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ManifestSummary {
    pub total_discovered: usize,
    pub total_extracted: usize,
    pub total_missing: usize,
    pub total_failed: usize,
    pub total_bytes_extracted: u64,
    pub materials_extracted: usize,
    pub textures_extracted: usize,
    pub models_extracted: usize,
    pub sounds_extracted: usize,
    pub particles_extracted: usize,
    pub instances_extracted: usize,
    pub auxiliary_extracted: usize,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Manifest {
    pub map_name: String,
    pub records: BTreeMap<String, AssetRecord>,
}

impl Manifest {
    pub fn new(map_name: &str) -> Self {
        Self {
            map_name: map_name.to_string(),
            records: BTreeMap::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record(
        &mut self,
        virtual_path: VirtualAssetPath,
        kind: AssetKind,
        source_path: Option<PathBuf>,
        dest_path: Option<PathBuf>,
        status: AssetStatus,
        size_bytes: u64,
        reason: &str,
    ) {
        let key = virtual_path.as_str().to_lowercase();
        self.records.insert(
            key,
            AssetRecord {
                virtual_path,
                kind,
                source_path,
                dest_path,
                status,
                size_bytes,
                reason: reason.to_string(),
            },
        );
    }

    pub fn summary(&self) -> ManifestSummary {
        let mut s = ManifestSummary {
            total_discovered: self.records.len(),
            ..Default::default()
        };

        for record in self.records.values() {
            match &record.status {
                AssetStatus::Extracted => {
                    s.total_extracted += 1;
                    s.total_bytes_extracted += record.size_bytes;
                    match record.kind {
                        AssetKind::Material => s.materials_extracted += 1,
                        AssetKind::Texture => s.textures_extracted += 1,
                        AssetKind::Model => s.models_extracted += 1,
                        AssetKind::Sound => s.sounds_extracted += 1,
                        AssetKind::Particle => s.particles_extracted += 1,
                        AssetKind::Instance => s.instances_extracted += 1,
                        AssetKind::Auxiliary => s.auxiliary_extracted += 1,
                        AssetKind::Other | AssetKind::Script => {}
                    }
                }
                AssetStatus::Missing => s.total_missing += 1,
                AssetStatus::Failed(_) => s.total_failed += 1,
                _ => {}
            }
        }

        s
    }

    /// Generates bspzip -addorupdatelist file entries exclusively for auxiliary companion files.
    /// In Garry's Mod / Source engine mapping, models, textures, materials and sounds
    /// must remain as loose files in the addon directory for distribution (Workshop / FastDL),
    /// while only auxiliary map files (.ain nodegraph, .nav mesh, map thumbnail, particles list, soundscapes)
    /// are embedded into the BSP pakfile.
    pub fn generate_bspzip_list(&self) -> Vec<String> {
        let mut lines = Vec::new();
        for record in self.records.values() {
            if record.status == AssetStatus::Extracted && record.kind == AssetKind::Auxiliary {
                if let Some(dest) = &record.dest_path {
                    if dest.exists() {
                        lines.push(record.virtual_path.as_str().to_string());
                        lines.push(dest.to_string_lossy().to_string());
                    }
                }
            }
        }
        lines
    }

    pub fn write_json(&self, path: &Path) -> std::io::Result<()> {
        let file = fs::File::create(path)?;
        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }
}
