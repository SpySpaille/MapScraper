use std::collections::HashSet;
use crate::utils::{clean_sound_path, normalize_source_path};

#[derive(Default, Debug)]
pub struct VmfAssets {
    pub materials: HashSet<String>,
    pub models: HashSet<String>,
    pub sounds: HashSet<String>,
    pub direct_vtfs: HashSet<String>,
    pub instances: HashSet<String>,
    pub has_soundscapes: bool,
}

pub fn parse_vmf(vmf_content: &str, sound_getters: &HashSet<String>) -> VmfAssets {
    let mut assets = VmfAssets::default();

    // 1. Skybox
    let sky_re = regex::Regex::new(r#"(?i)"skyname"\s*"([^"]+)""#).unwrap();
    for cap in sky_re.captures_iter(vmf_content) {
        let sky = cap[1].trim();
        if !sky.is_empty() {
            assets.materials.insert(format!("skybox/{}up", sky));
            assets.materials.insert(format!("skybox/{}dn", sky));
            assets.materials.insert(format!("skybox/{}lf", sky));
            assets.materials.insert(format!("skybox/{}rt", sky));
            assets.materials.insert(format!("skybox/{}ft", sky));
            assets.materials.insert(format!("skybox/{}bk", sky));
        }
    }

    // 2. Brush and entity materials
    let mat_keys = ["material", "texture", "detailmaterial", "overlayname", "bottommaterial"];
    for key in mat_keys {
        let re = regex::Regex::new(&format!(r#"(?i)"{}"\s*"([^"]+)""#, key)).unwrap();
        for cap in re.captures_iter(vmf_content) {
            let mut mat = normalize_source_path(&cap[1]);
            let lower = mat.to_lowercase();
            if !lower.starts_with("tools/") && !lower.starts_with("maps/") && !lower.is_empty() {
                if lower.ends_with(".vmt") {
                    mat = mat[..mat.len() - 4].to_string();
                }
                assets.materials.insert(mat);
            }
        }
    }

    // 3. Direct VTF (e.g. startexture)
    let star_re = regex::Regex::new(r#"(?i)"startexture"\s*"([^"]+)""#).unwrap();
    for cap in star_re.captures_iter(vmf_content) {
        let mut vtf = normalize_source_path(&cap[1]);
        if vtf.to_lowercase().ends_with(".vtf") {
            vtf = vtf[..vtf.len() - 4].to_string();
        }
        if !vtf.is_empty() {
            assets.direct_vtfs.insert(vtf);
        }
    }

    // 4. Models
    let mdl_re = regex::Regex::new(r#"(?i)"model"\s*"([^"]+)""#).unwrap();
    for cap in mdl_re.captures_iter(vmf_content) {
        let mdl = normalize_source_path(&cap[1]);
        if mdl.to_lowercase().ends_with(".mdl") {
            assets.models.insert(mdl);
        }
    }

    // 5. Sounds
    let mut all_sound_keys: Vec<String> = vec![
        "message".to_string(),
        "noise1".to_string(),
        "noise2".to_string(),
        "closesound".to_string(),
        "startclosesound".to_string(),
        "shootsound".to_string(),
    ];
    for sg in sound_getters {
        if !all_sound_keys.contains(sg) {
            all_sound_keys.push(sg.clone());
        }
    }

    for key in all_sound_keys {
        let re = regex::Regex::new(&format!(r#"(?i)"{}"\s*"([^"]+)""#, key)).unwrap();
        for cap in re.captures_iter(vmf_content) {
            let clean = clean_sound_path(&cap[1]);
            let lower = clean.to_lowercase();
            if lower.ends_with(".wav") || lower.ends_with(".mp3") || lower.ends_with(".ogg") {
                assets.sounds.insert(clean);
            }
        }
    }

    // 6. Func_instance extraction
    let entity_re = regex::Regex::new(r"(?is)entity\s*\{([^}]+)\}").unwrap();
    for cap in entity_re.captures_iter(vmf_content) {
        let block = &cap[1];
        if block.to_lowercase().contains("\"classname\" \"func_instance\"")
            || block.to_lowercase().contains("\"classname\"\t\"func_instance\"") {
            let file_re = regex::Regex::new(r#"(?i)"(?:file|vmf)"\s*"([^"]+)""#).unwrap();
            if let Some(fcap) = file_re.captures(block) {
                let mut inst_file = normalize_source_path(&fcap[1]);
                if !inst_file.to_lowercase().ends_with(".vmf") {
                    inst_file.push_str(".vmf");
                }
                assets.instances.insert(inst_file);
            }
        }
    }

    assets.has_soundscapes = vmf_content.to_lowercase().contains("soundscape");

    assets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vmf() {
        let vmf = r#"
            world {
                "skyname" "sky_dust"
                solid {
                    side {
                        "material" "BRICK/WALL01"
                    }
                }
            }
            entity {
                "classname" "prop_static"
                "model" "models/props/chair.mdl"
            }
            entity {
                "classname" "ambient_generic"
                "message" ")ambient/wind.wav"
            }
            entity {
                "classname" "func_instance"
                "file" "instances/door.vmf"
            }
        "#;
        let sound_getters = HashSet::new();
        let assets = parse_vmf(vmf, &sound_getters);

        assert!(assets.materials.contains("skybox/sky_dustup"));
        assert!(assets.materials.contains("skybox/sky_dustbk"));
        assert!(assets.materials.contains("BRICK/WALL01"));
        assert!(assets.models.contains("models/props/chair.mdl"));
        assert!(assets.sounds.contains("ambient/wind.wav"));
        assert!(assets.instances.contains("instances/door.vmf"));
    }
}
