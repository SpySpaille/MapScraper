use crate::keyvalues::{parse_keyvalues, KVValue};
use crate::mounts::SearchPathManager;
use crate::utils::clean_sound_path;
use std::collections::HashMap;

#[derive(Default, Debug)]
pub struct SoundscriptResolver {
    // event_name lowercase -> list of wave file paths
    pub sound_map: HashMap<String, Vec<String>>,
}

impl SoundscriptResolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_from_search_paths(&mut self, search_paths: &SearchPathManager) {
        // 1. Try reading scripts/game_sounds_manifest.txt
        if let Some(manifest_asset) = search_paths.resolve_asset("scripts/game_sounds_manifest.txt")
        {
            if let Ok(bytes) = search_paths.read_asset(&manifest_asset) {
                let content = String::from_utf8_lossy(&bytes);
                if let Ok(doc) = parse_keyvalues(&content) {
                    for (_root_key, root_val) in doc.root_entries {
                        if let KVValue::Block(entries) = root_val {
                            for (k, v) in entries {
                                if k.to_lowercase() == "precache_file" {
                                    if let KVValue::String(sub_file) = v {
                                        self.load_single_soundscript(&sub_file, search_paths);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. Also check common Garry's Mod soundscripts
        let common_scripts = [
            "scripts/game_sounds.txt",
            "scripts/game_sounds_weapons.txt",
            "scripts/game_sounds_ambient.txt",
            "scripts/game_sounds_world.txt",
            "scripts/game_sounds_physics.txt",
            "scripts/game_sounds_vehicles.txt",
            "scripts/game_sounds_player.txt",
        ];

        for script in common_scripts {
            self.load_single_soundscript(script, search_paths);
        }
    }

    pub fn load_single_soundscript(&mut self, script_path: &str, search_paths: &SearchPathManager) {
        let clean_path = script_path.trim().replace('\\', "/");
        let Some(asset) = search_paths.resolve_asset(&clean_path) else {
            return;
        };

        let Ok(bytes) = search_paths.read_asset(&asset) else {
            return;
        };

        let content = String::from_utf8_lossy(&bytes);
        let Ok(doc) = parse_keyvalues(&content) else {
            return;
        };

        for (event_name, event_val) in doc.root_entries {
            let key = event_name.to_lowercase();
            if let KVValue::Block(entries) = event_val {
                let mut waves = Vec::new();
                for (k, v) in entries {
                    let k_lower = k.to_lowercase();
                    if k_lower == "wave" {
                        if let KVValue::String(w) = v {
                            let clean = clean_sound_path(&w);
                            if !clean.is_empty() && !waves.contains(&clean) {
                                waves.push(clean);
                            }
                        }
                    } else if k_lower == "rndwave" {
                        if let KVValue::Block(rnd_entries) = v {
                            for (rk, rv) in rnd_entries {
                                if rk.to_lowercase() == "wave" {
                                    if let KVValue::String(rw) = rv {
                                        let clean = clean_sound_path(&rw);
                                        if !clean.is_empty() && !waves.contains(&clean) {
                                            waves.push(clean);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if !waves.is_empty() {
                    self.sound_map.entry(key).or_default().extend(waves);
                }
            }
        }
    }

    pub fn resolve_sound(&self, sound_ref: &str) -> Vec<String> {
        let clean = clean_sound_path(sound_ref);
        let lower = clean.to_lowercase();

        // If it's already a direct sound file path
        if lower.ends_with(".wav") || lower.ends_with(".mp3") || lower.ends_with(".ogg") {
            return vec![clean];
        }

        // Check if event name is in soundscripts
        if let Some(waves) = self.sound_map.get(&lower) {
            return waves.clone();
        }

        // Return empty or candidate if not resolvable
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_soundscript_content() {
        let script = r#"
            "Weapon_AR2.Single"
            {
                "channel" "CHAN_WEAPON"
                "volume" "1.0"
                "wave" "^weapons/ar2/fire1.wav"
            }
            "Door.Open"
            {
                "channel" "CHAN_STATIC"
                "rndwave"
                {
                    "wave" "doors/door1.wav"
                    "wave" "doors/door2.wav"
                }
            }
        "#;
        let doc = parse_keyvalues(script).unwrap();
        let mut resolver = SoundscriptResolver::new();

        for (event_name, event_val) in doc.root_entries {
            let key = event_name.to_lowercase();
            if let KVValue::Block(entries) = event_val {
                let mut waves = Vec::new();
                for (k, v) in entries {
                    if k.to_lowercase() == "wave" {
                        if let KVValue::String(w) = v {
                            waves.push(clean_sound_path(&w));
                        }
                    } else if k.to_lowercase() == "rndwave" {
                        if let KVValue::Block(rnd_entries) = v {
                            for (rk, rv) in rnd_entries {
                                if rk.to_lowercase() == "wave" {
                                    if let KVValue::String(rw) = rv {
                                        waves.push(clean_sound_path(&rw));
                                    }
                                }
                            }
                        }
                    }
                }
                resolver.sound_map.insert(key, waves);
            }
        }

        let ar2 = resolver.resolve_sound("Weapon_AR2.Single");
        assert_eq!(ar2, vec!["weapons/ar2/fire1.wav"]);

        let door = resolver.resolve_sound("Door.Open");
        assert_eq!(door, vec!["doors/door1.wav", "doors/door2.wav"]);
    }
}
