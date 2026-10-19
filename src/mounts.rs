use std::fs;
use std::path::{Path, PathBuf};
use crate::utils::{clean_quotes, normalize_source_path};

#[derive(Debug, Clone)]
pub struct SearchPath {
    pub label: String,
    pub path: PathBuf,
}

pub struct SearchPathManager {
    pub primary_path: PathBuf,
    pub all_paths: Vec<SearchPath>,
}

impl SearchPathManager {
    pub fn new<P: AsRef<Path>>(game_path: P, custom_mounts: &[String]) -> Self {
        let primary_path = game_path.as_ref().to_path_buf();
        let mut all_paths = Vec::new();
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

        // 1. Primary Game Path
        add_path(primary_path.clone(), "Primary Game");

        // 2. Scan mount.cfg
        let mount_cfg = primary_path.join("cfg").join("mount.cfg");
        if mount_cfg.exists() {
            if let Ok(content) = fs::read_to_string(&mount_cfg) {
                let re = regex::Regex::new(r#""([^"]+)"\s+"([^"]+)""#).unwrap();
                for cap in re.captures_iter(&content) {
                    let name = &cap[1];
                    let raw_path = &cap[2];
                    if name.to_lowercase() != "mountcfg" {
                        let candidate = PathBuf::from(raw_path);
                        if candidate.exists() {
                            add_path(candidate, &format!("mount.cfg ({})", name));
                        }
                    }
                }
            }
        }

        // 3. Scan gameinfo.txt
        let gameinfo = primary_path.join("gameinfo.txt");
        if gameinfo.exists() {
            if let Ok(content) = fs::read_to_string(&gameinfo) {
                if let Some(sp_start) = content.find("SearchPaths") {
                    let block = &content[sp_start..];
                    if let Some(brace_open) = block.find('{') {
                        if let Some(brace_close) = block.find('}') {
                            let inside = &block[brace_open + 1..brace_close];
                            for line in inside.lines() {
                                let line = line.split("//").next().unwrap_or("").trim();
                                let parts: Vec<&str> = line.split_whitespace().collect();
                                if parts.len() >= 2 {
                                    let target = clean_quotes(parts[1]);
                                    if !target.contains('|') {
                                        if let Some(parent) = primary_path.parent() {
                                            let candidate = parent.join(&target);
                                            if candidate.exists() {
                                                add_path(candidate, &format!("gameinfo.txt ({})", target));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 4. Scan loose addons (garrysmod/addons/*)
        let addons_dir = primary_path.join("addons");
        if addons_dir.exists() {
            if let Ok(entries) = fs::read_dir(&addons_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_dir() && (p.join("materials").exists() || p.join("models").exists()) {
                        let name = entry.file_name().to_string_lossy().to_string();
                        add_path(p, &format!("addon ({})", name));
                    }
                }
            }
        }

        // 5. Custom Mounts
        for cm in custom_mounts {
            let p = PathBuf::from(clean_quotes(cm));
            if p.exists() {
                add_path(p, "Custom Mount");
            }
        }

        Self {
            primary_path,
            all_paths,
        }
    }

    /// Resolves a relative asset path across all mounted directories.
    pub fn resolve(&self, relative_asset_path: &str) -> Option<PathBuf> {
        let norm = normalize_source_path(relative_asset_path);
        let path_obj = Path::new(&norm);

        for sp in &self.all_paths {
            let direct = sp.path.join(path_obj);
            if direct.exists() {
                return Some(direct);
            }

            // Case-insensitive fallback if needed on Windows/Linux
            let candidate = find_file_case_insensitive(&sp.path, path_obj);
            if let Some(found) = candidate {
                return Some(found);
            }
        }

        None
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

        fs::write(primary.join("materials").join("primary_mat").join("wood.vmt"), "test").unwrap();
        fs::write(mount_css.join("materials").join("cstrike_mat").join("de_dust.vmt"), "test").unwrap();

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
