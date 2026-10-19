use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use crate::mounts::SearchPathManager;
use crate::utils::normalize_source_path;
use crate::vmf::{parse_vmf, VmfAssets};

pub struct InstanceFile {
    pub source_path: PathBuf,
    pub relative_path: String,
}

pub struct InstanceResolver<'a> {
    search_paths: &'a SearchPathManager,
    sound_getters: &'a HashSet<String>,
    visited: HashSet<String>,
}

impl<'a> InstanceResolver<'a> {
    pub fn new(search_paths: &'a SearchPathManager, sound_getters: &'a HashSet<String>) -> Self {
        Self {
            search_paths,
            sound_getters,
            visited: HashSet::new(),
        }
    }

    fn resolve_instance_path(&self, rel_path: &str, parent_vmf: &Path) -> Option<PathBuf> {
        let clean = normalize_source_path(rel_path);
        let parent_dir = parent_vmf.parent().unwrap_or_else(|| Path::new(""));

        // Candidates:
        let mut candidates = vec![
            parent_dir.join(&clean),
            self.search_paths.primary_path.join("maps").join(&clean),
            self.search_paths.primary_path.join(&clean),
        ];

        for sp in &self.search_paths.all_paths {
            candidates.push(sp.path.join("maps").join(&clean));
            candidates.push(sp.path.join(&clean));
        }

        for cand in candidates {
            if cand.exists() {
                return Some(cand);
            }
        }

        None
    }

    pub fn process_instances(
        &mut self,
        initial_instances: &HashSet<String>,
        root_vmf: &Path,
    ) -> (VmfAssets, Vec<InstanceFile>) {
        let mut merged = VmfAssets::default();
        let mut instance_files = Vec::new();
        let mut queue: Vec<(String, PathBuf)> = initial_instances
            .iter()
            .map(|inst| (inst.clone(), root_vmf.to_path_buf()))
            .collect();

        while let Some((rel_path, parent_path)) = queue.pop() {
            let Some(resolved) = self.resolve_instance_path(&rel_path, &parent_path) else {
                continue;
            };

            let key = resolved.to_string_lossy().to_lowercase();
            if self.visited.contains(&key) {
                continue; // Avoid cycle
            }
            self.visited.insert(key);

            instance_files.push(InstanceFile {
                source_path: resolved.clone(),
                relative_path: normalize_source_path(&rel_path),
            });

            if let Ok(content) = fs::read_to_string(&resolved) {
                let parsed = parse_vmf(&content, self.sound_getters);

                merged.materials.extend(parsed.materials);
                merged.models.extend(parsed.models);
                merged.sounds.extend(parsed.sounds);
                merged.direct_vtfs.extend(parsed.direct_vtfs);

                for child_inst in parsed.instances {
                    queue.push((child_inst, resolved.clone()));
                }
            }
        }

        (merged, instance_files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recursive_instances_and_cycle_avoidance() {
        let temp_dir = std::env::temp_dir().join("mapscraper_test_inst");
        let _ = fs::remove_dir_all(&temp_dir);
        let game_dir = temp_dir.join("game");
        let inst_dir = game_dir.join("maps").join("instances");
        fs::create_dir_all(&inst_dir).unwrap();

        let inst1_path = inst_dir.join("inst1.vmf");
        let inst2_path = inst_dir.join("inst2.vmf");

        // inst1 references inst2
        fs::write(
            &inst1_path,
            r#"
            entity {
                "classname" "func_instance"
                "file" "instances/inst2.vmf"
            }
            entity {
                "classname" "prop_static"
                "model" "models/inst1_model.mdl"
            }
            "#,
        ).unwrap();

        // inst2 references inst1 (circular cycle test!)
        fs::write(
            &inst2_path,
            r#"
            entity {
                "classname" "func_instance"
                "file" "instances/inst1.vmf"
            }
            entity {
                "classname" "prop_static"
                "model" "models/inst2_model.mdl"
            }
            "#,
        ).unwrap();

        let root_vmf = game_dir.join("maps").join("root.vmf");
        fs::write(&root_vmf, "root").unwrap();

        let spm = SearchPathManager::new(&game_dir, &[]);
        let sound_getters = HashSet::new();
        let mut resolver = InstanceResolver::new(&spm, &sound_getters);

        let mut initial = HashSet::new();
        initial.insert("instances/inst1.vmf".to_string());

        let (assets, files) = resolver.process_instances(&initial, &root_vmf);
        assert_eq!(files.len(), 2, "Should resolve both instances without infinite loop");
        assert!(assets.models.contains("models/inst1_model.mdl"));
        assert!(assets.models.contains("models/inst2_model.mdl"));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
