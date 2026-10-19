use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::bsp::BspArchive;
use crate::gma::GmaArchive;
use crate::utils::{clean_quotes, normalize_source_path, safe_copy_file};
use crate::vpk_fs::VpkArchive;

#[derive(Debug, Clone)]
pub struct SearchPath {
    pub label: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub enum ResolvedAsset {
    Loose {
        path: PathBuf,
        label: String,
    },
    Bsp {
        bsp_index: usize,
        rel_path: String,
        label: String,
    },
    Vpk {
        vpk_index: usize,
        rel_path: String,
        label: String,
    },
    Gma {
        gma_index: usize,
        rel_path: String,
        label: String,
    },
}

impl ResolvedAsset {
    pub fn label(&self) -> &str {
        match self {
            ResolvedAsset::Loose { label, .. } => label,
            ResolvedAsset::Bsp { label, .. } => label,
            ResolvedAsset::Vpk { label, .. } => label,
            ResolvedAsset::Gma { label, .. } => label,
        }
    }

    pub fn loose_path(&self) -> Option<&Path> {
        match self {
            ResolvedAsset::Loose { path, .. } => Some(path.as_path()),
            _ => None,
        }
    }
}

pub struct SearchPathManager {
    pub primary_path: PathBuf,
    pub all_paths: Vec<SearchPath>,
    pub bsps: Vec<BspArchive>,
    pub vpks: Vec<VpkArchive>,
    pub gmas: Vec<GmaArchive>,
}

impl SearchPathManager {
    pub fn new<P: AsRef<Path>>(game_path: P, custom_mounts: &[String]) -> Self {
        let primary_path = game_path.as_ref().to_path_buf();
        let mut all_paths = Vec::new();
        let mut vpks = Vec::new();
        let mut gmas = Vec::new();
        let mut seen = std::collections::HashSet::new();

        let mut add_path = |p: PathBuf, label: &str| {
            if let Ok(canon) = fs::canonicalize(&p) {
                let key = canon.to_string_lossy().to_lowercase();
                if !seen.contains(&key) && p.exists() {
                    seen.insert(key);
                    all_paths.push(SearchPath {
                        label: label.to_string(),
                        path: p,
                    });
                }
            } else if p.exists() {
                let key = p.to_string_lossy().to_lowercase();
                if !seen.contains(&key) {
                    seen.insert(key);
                    all_paths.push(SearchPath {
                        label: label.to_string(),
                        path: p,
                    });
                }
            }
        };

        let mut add_vpk = |vpk_path: PathBuf, label: &str| {
            if vpk_path.exists() {
                if let Some(archive) = VpkArchive::open(&vpk_path, label) {
                    vpks.push(archive);
                }
            }
        };

        let mut add_gma = |gma_path: PathBuf, label: &str| {
            if gma_path.exists() {
                if let Ok(archive) = GmaArchive::open(&gma_path, label) {
                    gmas.push(archive);
                }
            }
        };

        // 1. Primary Game Path
        add_path(primary_path.clone(), "Primary Game");

        // Scan primary VPKs (e.g. garrysmod_dir.vpk)
        if let Ok(entries) = fs::read_dir(&primary_path) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    let name = p
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_lowercase();
                    if name.ends_with("_dir.vpk") {
                        add_vpk(p, &format!("Primary VPK ({})", name));
                    }
                }
            }
        }

        // 2. Scan mount.cfg
        let mount_cfg = primary_path.join("cfg").join("mount.cfg");
        if mount_cfg.exists() {
            if let Ok(bytes) = fs::read(&mount_cfg) {
                let content = String::from_utf8_lossy(&bytes);
                let re = regex::Regex::new(r#""([^"]+)"\s+"([^"]+)""#).unwrap();
                for cap in re.captures_iter(&content) {
                    let name = &cap[1];
                    let raw_path = &cap[2];
                    if name.to_lowercase() != "mountcfg" {
                        let candidate = PathBuf::from(raw_path);
                        if candidate.exists() {
                            add_path(candidate.clone(), &format!("mount.cfg ({})", name));

                            // Scan mounted game VPKs
                            if let Ok(entries) = fs::read_dir(&candidate) {
                                for entry in entries.flatten() {
                                    let vp = entry.path();
                                    if vp.is_file() {
                                        let vname = vp
                                            .file_name()
                                            .unwrap_or_default()
                                            .to_string_lossy()
                                            .to_lowercase();
                                        if vname.ends_with("_dir.vpk")
                                            || vname.ends_with("_pak_dir.vpk")
                                        {
                                            add_vpk(
                                                vp,
                                                &format!("Mounted VPK ({}/{})", name, vname),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 3. Scan gameinfo.txt with Source engine macro expansions
        let gameinfo = primary_path.join("gameinfo.txt");
        if gameinfo.exists() {
            if let Ok(bytes) = fs::read(&gameinfo) {
                let content = String::from_utf8_lossy(&bytes);
                if let Some(sp_start) = content.find("SearchPaths") {
                    let block = &content[sp_start..];
                    if let Some(brace_open) = block.find('{') {
                        if let Some(brace_close) = block.find('}') {
                            let inside = &block[brace_open + 1..brace_close];
                            for line in inside.lines() {
                                let line = line.split("//").next().unwrap_or("").trim();
                                let parts: Vec<&str> = line.split_whitespace().collect();
                                if parts.len() >= 2 {
                                    let raw_target = clean_quotes(parts[1]);
                                    let target_lower = raw_target.to_lowercase();

                                    // Macro resolution:
                                    // |gameinfo_path| -> primary_path
                                    // |all_source_engine_paths| -> parent of primary_path
                                    let resolved_dir = if let Some(stripped) =
                                        target_lower.strip_prefix("|gameinfo_path|")
                                    {
                                        let sub = stripped.trim_start_matches(['/', '\\']);
                                        primary_path.join(sub)
                                    } else if let Some(stripped) =
                                        target_lower.strip_prefix("|all_source_engine_paths|")
                                    {
                                        let sub = stripped.trim_start_matches(['/', '\\']);
                                        primary_path
                                            .parent()
                                            .map(|p| p.join(sub))
                                            .unwrap_or_else(|| PathBuf::from(sub))
                                    } else if let Some(parent) = primary_path.parent() {
                                        parent.join(&raw_target)
                                    } else {
                                        PathBuf::from(&raw_target)
                                    };

                                    // Check wildcard custom/*
                                    if raw_target.ends_with("/*") || raw_target.ends_with("\\*") {
                                        let base_dir =
                                            resolved_dir.parent().unwrap_or(&resolved_dir);
                                        if let Ok(entries) = fs::read_dir(base_dir) {
                                            for entry in entries.flatten() {
                                                let ep = entry.path();
                                                if ep.is_dir() {
                                                    add_path(ep, "gameinfo.txt custom/*");
                                                } else if ep.is_file()
                                                    && ep.extension().is_some_and(|ext| {
                                                        ext.eq_ignore_ascii_case("vpk")
                                                    })
                                                {
                                                    add_vpk(ep, "gameinfo.txt custom/*.vpk");
                                                }
                                            }
                                        }
                                    } else if resolved_dir.exists() {
                                        if resolved_dir.is_dir() {
                                            add_path(
                                                resolved_dir,
                                                &format!("gameinfo.txt ({})", raw_target),
                                            );
                                        } else if resolved_dir.is_file()
                                            && raw_target.ends_with(".vpk")
                                        {
                                            add_vpk(
                                                resolved_dir,
                                                &format!("gameinfo.txt ({})", raw_target),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 4. Scan Garry's Mod addons (loose dirs and .gma files)
        let addons_dir = primary_path.join("addons");
        if addons_dir.exists() {
            if let Ok(entries) = fs::read_dir(&addons_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    let name = entry.file_name().to_string_lossy().to_string();
                    if p.is_dir() {
                        // Check all Source asset folders
                        let has_assets = p.join("materials").exists()
                            || p.join("models").exists()
                            || p.join("sound").exists()
                            || p.join("particles").exists()
                            || p.join("scripts").exists()
                            || p.join("resource").exists();
                        if has_assets {
                            add_path(p, &format!("addon ({})", name));
                        }
                    } else if p.is_file()
                        && p.extension()
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("gma"))
                    {
                        add_gma(p, &format!("GMA Addon ({})", name));
                    }
                }
            }
        }

        // 5. Custom Mounts
        for cm in custom_mounts {
            let p = PathBuf::from(clean_quotes(cm));
            if p.exists() {
                if p.is_dir() {
                    add_path(p, "Custom Mount");
                } else if p.is_file() {
                    let lower = p.to_string_lossy().to_lowercase();
                    if lower.ends_with(".vpk") {
                        add_vpk(p, "Custom VPK Mount");
                    } else if lower.ends_with(".gma") {
                        add_gma(p, "Custom GMA Mount");
                    }
                }
            }
        }

        Self {
            primary_path,
            all_paths,
            bsps: Vec::new(),
            vpks,
            gmas,
        }
    }

    /// Mounts a compiled BSP file to resolve pakfile assets (e.g. WVT patches, cubemaps, embedded textures).
    pub fn mount_bsp<P: AsRef<Path>>(&mut self, bsp_path: P) -> bool {
        let p = bsp_path.as_ref();
        if p.exists() && p.is_file() {
            if let Ok(archive) = BspArchive::open(p) {
                self.bsps.push(archive);
                return true;
            }
        }
        false
    }

    /// Resolves an asset path across loose directories, BSP pakfiles, GMA addons, and VPK archives.
    pub fn resolve_asset(&self, relative_asset_path: &str) -> Option<ResolvedAsset> {
        let norm = normalize_source_path(relative_asset_path);
        let path_obj = Path::new(&norm);

        // 1. Check loose directories
        for sp in &self.all_paths {
            let direct = sp.path.join(path_obj);
            if direct.exists() {
                return Some(ResolvedAsset::Loose {
                    path: direct,
                    label: sp.label.clone(),
                });
            }

            // Case-insensitive fallback
            if let Some(found) = find_file_case_insensitive(&sp.path, path_obj) {
                return Some(ResolvedAsset::Loose {
                    path: found,
                    label: sp.label.clone(),
                });
            }
        }

        // 2. Check BSP pakfiles
        for (idx, bsp) in self.bsps.iter().enumerate() {
            if bsp.has_file(&norm) {
                return Some(ResolvedAsset::Bsp {
                    bsp_index: idx,
                    rel_path: norm.clone(),
                    label: bsp.label.clone(),
                });
            }
        }

        // 3. Check GMA addons
        for (idx, gma) in self.gmas.iter().enumerate() {
            if gma.has_file(&norm) {
                return Some(ResolvedAsset::Gma {
                    gma_index: idx,
                    rel_path: norm.clone(),
                    label: gma.label.clone(),
                });
            }
        }

        // 4. Check VPK archives
        for (idx, vpk) in self.vpks.iter().enumerate() {
            if vpk.has_file(&norm) {
                return Some(ResolvedAsset::Vpk {
                    vpk_index: idx,
                    rel_path: norm.clone(),
                    label: vpk.label.clone(),
                });
            }
        }

        None
    }

    /// Resolves a relative asset path to a disk PathBuf (if available in loose directories).
    pub fn resolve(&self, relative_asset_path: &str) -> Option<PathBuf> {
        let norm = normalize_source_path(relative_asset_path);
        let path_obj = Path::new(&norm);

        for sp in &self.all_paths {
            let direct = sp.path.join(path_obj);
            if direct.exists() {
                return Some(direct);
            }

            if let Some(found) = find_file_case_insensitive(&sp.path, path_obj) {
                return Some(found);
            }
        }

        None
    }

    /// Extracts an asset directly to destination disk path.
    pub fn extract_asset(&self, asset: &ResolvedAsset, dest_path: &Path) -> io::Result<u64> {
        match asset {
            ResolvedAsset::Loose { path, .. } => safe_copy_file(path, dest_path),
            ResolvedAsset::Bsp {
                bsp_index,
                rel_path,
                ..
            } => {
                if let Some(bsp) = self.bsps.get(*bsp_index) {
                    bsp.extract_file(rel_path, dest_path)
                } else {
                    Err(io::Error::new(io::ErrorKind::NotFound, "BSP index invalid"))
                }
            }
            ResolvedAsset::Vpk {
                vpk_index,
                rel_path,
                ..
            } => {
                if let Some(vpk) = self.vpks.get(*vpk_index) {
                    vpk.extract_file(rel_path, dest_path)
                } else {
                    Err(io::Error::new(io::ErrorKind::NotFound, "VPK index invalid"))
                }
            }
            ResolvedAsset::Gma {
                gma_index,
                rel_path,
                ..
            } => {
                if let Some(gma) = self.gmas.get(*gma_index) {
                    let bytes = gma.read_file(rel_path)?;
                    if let Some(parent) = dest_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(dest_path, &bytes)?;
                    Ok(bytes.len() as u64)
                } else {
                    Err(io::Error::new(io::ErrorKind::NotFound, "GMA index invalid"))
                }
            }
        }
    }

    /// Reads asset data into memory.
    pub fn read_asset(&self, asset: &ResolvedAsset) -> io::Result<Vec<u8>> {
        match asset {
            ResolvedAsset::Loose { path, .. } => fs::read(path),
            ResolvedAsset::Bsp {
                bsp_index,
                rel_path,
                ..
            } => {
                if let Some(bsp) = self.bsps.get(*bsp_index) {
                    bsp.read_file(rel_path)
                } else {
                    Err(io::Error::new(io::ErrorKind::NotFound, "BSP index invalid"))
                }
            }
            ResolvedAsset::Vpk {
                vpk_index,
                rel_path,
                ..
            } => {
                if let Some(vpk) = self.vpks.get(*vpk_index) {
                    vpk.read_file(rel_path)
                } else {
                    Err(io::Error::new(io::ErrorKind::NotFound, "VPK index invalid"))
                }
            }
            ResolvedAsset::Gma {
                gma_index,
                rel_path,
                ..
            } => {
                if let Some(gma) = self.gmas.get(*gma_index) {
                    gma.read_file(rel_path)
                } else {
                    Err(io::Error::new(io::ErrorKind::NotFound, "GMA index invalid"))
                }
            }
        }
    }
}

fn find_file_case_insensitive(base: &Path, rel: &Path) -> Option<PathBuf> {
    let mut current = base.to_path_buf();
    for comp in rel.components() {
        let comp_str = comp.as_os_str().to_string_lossy().to_lowercase();
        let mut matched = false;
        if let Ok(entries) = fs::read_dir(&current) {
            for entry in entries.flatten() {
                if entry.file_name().to_string_lossy().to_lowercase() == comp_str {
                    current = entry.path();
                    matched = true;
                    break;
                }
            }
        }
        if !matched {
            return None;
        }
    }
    if current.exists() {
        Some(current)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_paths_and_mounts() {
        let temp_dir = std::env::temp_dir().join("mapscraper_test_mounts");
        let _ = fs::remove_dir_all(&temp_dir);
        let primary = temp_dir.join("primary");
        let mount_css = temp_dir.join("cstrike");

        fs::create_dir_all(primary.join("cfg")).unwrap();
        fs::create_dir_all(primary.join("materials").join("primary_mat")).unwrap();
        fs::create_dir_all(mount_css.join("materials").join("cstrike_mat")).unwrap();

        fs::write(
            primary
                .join("materials")
                .join("primary_mat")
                .join("wood.vmt"),
            "test",
        )
        .unwrap();
        fs::write(
            mount_css
                .join("materials")
                .join("cstrike_mat")
                .join("de_dust.vmt"),
            "test",
        )
        .unwrap();

        // mount.cfg
        let mount_cfg = format!(
            "\"mountcfg\"\n{{\n    \"cstrike\" \"{}\"\n}}\n",
            mount_css.display().to_string().replace('\\', "\\\\")
        );
        fs::write(primary.join("cfg").join("mount.cfg"), mount_cfg).unwrap();

        let spm = SearchPathManager::new(&primary, &[]);
        assert_eq!(spm.all_paths.len(), 2);

        let res_primary = spm.resolve("materials/primary_mat/wood.vmt");
        assert!(res_primary.is_some());

        let res_css = spm.resolve("materials/cstrike_mat/de_dust.vmt");
        assert!(res_css.is_some());

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
