use crate::keyvalues::{parse_keyvalues, KVValue};
use crate::utils::{clean_sound_path, normalize_source_path};
use std::collections::HashSet;

#[derive(Default, Debug, Clone)]
pub struct VmfAssets {
    pub materials: HashSet<String>,
    pub models: HashSet<String>,
    pub sounds: HashSet<String>,
    pub direct_vtfs: HashSet<String>,
    pub instances: HashSet<String>,
    pub detail_vbsps: HashSet<String>,
    pub particle_effects: HashSet<String>,
    pub has_soundscapes: bool,
}

pub fn parse_vmf(vmf_content: &str, sound_getters: &HashSet<String>) -> VmfAssets {
    let mut assets = VmfAssets::default();

    if let Ok(doc) = parse_keyvalues(vmf_content) {
        for (root_key, root_val) in &doc.root_entries {
            traverse_vmf_block(root_key, root_val, &mut assets, sound_getters);
        }
    } else {
        // Fallback regex parser for non-standard/partially corrupt files
        fallback_regex_parse(vmf_content, &mut assets, sound_getters);
    }

    if vmf_content.to_lowercase().contains("soundscape") {
        assets.has_soundscapes = true;
    }

    assets
}

fn traverse_vmf_block(
    block_name: &str,
    val: &KVValue,
    assets: &mut VmfAssets,
    sound_getters: &HashSet<String>,
) {
    if let KVValue::Block(entries) = val {
        let is_entity = block_name.to_lowercase() == "entity";
        let mut classname = String::new();

        if is_entity {
            for (k, v) in entries {
                if k.to_lowercase() == "classname" {
                    if let KVValue::String(s) = v {
                        classname = s.to_lowercase();
                    }
                }
            }
        }

        let is_particle_sys = is_entity && classname == "info_particle_system";
        if is_particle_sys {
            for (k, v) in entries {
                if k.to_lowercase() == "effect_name" {
                    if let KVValue::String(s) = v {
                        let eff = s.trim().to_string();
                        if !eff.is_empty() {
                            assets.particle_effects.insert(eff);
                        }
                    }
                }
            }
        }

        // If func_instance entity, extract "file" or "vmf"
        if is_entity && classname == "func_instance" {
            for (k, v) in entries {
                let k_lower = k.to_lowercase();
                if k_lower == "file" || k_lower == "vmf" {
                    if let KVValue::String(s) = v {
                        let mut inst_file = normalize_source_path(s);
                        if !inst_file.to_lowercase().ends_with(".vmf") {
                            inst_file.push_str(".vmf");
                        }
                        if !inst_file.is_empty() {
                            assets.instances.insert(inst_file);
                        }
                    }
                }
            }
        }

        for (k, v) in entries {
            let k_lower = k.to_lowercase();
            match v {
                KVValue::String(s) => {
                    handle_kv_pair(&k_lower, s, assets, sound_getters);
                }
                KVValue::Block(_) => {
                    traverse_vmf_block(k, v, assets, sound_getters);
                }
            }
        }
    }
}

fn handle_kv_pair(key: &str, val: &str, assets: &mut VmfAssets, sound_getters: &HashSet<String>) {
    if val.trim().is_empty() {
        return;
    }

    // 1. Skybox
    if key == "skyname" {
        let sky = val.trim();
        if !sky.is_empty() {
            assets.materials.insert(format!("skybox/{}up", sky));
            assets.materials.insert(format!("skybox/{}dn", sky));
            assets.materials.insert(format!("skybox/{}lf", sky));
            assets.materials.insert(format!("skybox/{}rt", sky));
            assets.materials.insert(format!("skybox/{}ft", sky));
            assets.materials.insert(format!("skybox/{}bk", sky));
        }
        return;
    }

    // 2. Materials
    let mat_keys = [
        "material",
        "texture",
        "detailmaterial",
        "overlayname",
        "bottommaterial",
        "topmaterial",
    ];
    if mat_keys.contains(&key) {
        let mut mat = normalize_source_path(val);
        let lower = mat.to_lowercase();
        if !lower.starts_with("tools/") && !lower.is_empty() {
            if lower.ends_with(".vmt") {
                mat = mat[..mat.len() - 4].to_string();
            }
            assets.materials.insert(mat);
        }
        return;
    }

    // 3. Direct VTF (startexture)
    if key == "startexture" {
        let mut vtf = normalize_source_path(val);
        if vtf.to_lowercase().ends_with(".vtf") {
            vtf = vtf[..vtf.len() - 4].to_string();
        }
        if !vtf.is_empty() {
            assets.direct_vtfs.insert(vtf);
        }
        return;
    }

    // 3b. Detail VBSP (detailvbsp)
    if key == "detailvbsp" {
        let mut vbsp = normalize_source_path(val);
        if !vbsp.is_empty() {
            if !vbsp.to_lowercase().ends_with(".vbsp") {
                vbsp.push_str(".vbsp");
            }
            assets.detail_vbsps.insert(vbsp);
        }
        return;
    }

    // 4. Models
    if key == "model" {
        let mdl = normalize_source_path(val);
        if mdl.to_lowercase().ends_with(".mdl") {
            assets.models.insert(mdl);
        }
        return;
    }

    // 5. Sounds
    let sound_keys = [
        "message",
        "noise1",
        "noise2",
        "closesound",
        "startclosesound",
        "shootsound",
    ];
    if sound_keys.contains(&key) || sound_getters.contains(key) {
        let clean = clean_sound_path(val);
        let lower = clean.to_lowercase();
        // Support files with extensions (.wav, .mp3, .ogg) and soundscript event names (like Weapon_AR2.Single)
        let is_audio_file =
            lower.ends_with(".wav") || lower.ends_with(".mp3") || lower.ends_with(".ogg");
        let is_soundscript_name =
            !clean.contains('/') && !clean.contains('\\') && !clean.contains(' ');
        if (is_audio_file || is_soundscript_name) && !clean.is_empty() {
            assets.sounds.insert(clean);
        }
    }
}

fn fallback_regex_parse(
    vmf_content: &str,
    assets: &mut VmfAssets,
    _sound_getters: &HashSet<String>,
) {
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

    let mat_keys = [
        "material",
        "texture",
        "detailmaterial",
        "overlayname",
        "bottommaterial",
        "topmaterial",
    ];
    for key in mat_keys {
        let re = regex::Regex::new(&format!(r#"(?i)"{}"\s*"([^"]+)""#, key)).unwrap();
        for cap in re.captures_iter(vmf_content) {
            let mut mat = normalize_source_path(&cap[1]);
            let lower = mat.to_lowercase();
            if !lower.starts_with("tools/") && !lower.is_empty() {
                if lower.ends_with(".vmt") {
                    mat = mat[..mat.len() - 4].to_string();
                }
                assets.materials.insert(mat);
            }
        }
    }

    let mdl_re = regex::Regex::new(r#"(?i)"model"\s*"([^"]+)""#).unwrap();
    for cap in mdl_re.captures_iter(vmf_content) {
        let mdl = normalize_source_path(&cap[1]);
        if mdl.to_lowercase().ends_with(".mdl") {
            assets.models.insert(mdl);
        }
    }

    let vbsp_re = regex::Regex::new(r#"(?i)"detailvbsp"\s*"([^"]+)""#).unwrap();
    for cap in vbsp_re.captures_iter(vmf_content) {
        let mut vbsp = normalize_source_path(&cap[1]);
        if !vbsp.is_empty() {
            if !vbsp.to_lowercase().ends_with(".vbsp") {
                vbsp.push_str(".vbsp");
            }
            assets.detail_vbsps.insert(vbsp);
        }
    }

    let eff_re = regex::Regex::new(r#"(?i)"effect_name"\s*"([^"]+)""#).unwrap();
    for cap in eff_re.captures_iter(vmf_content) {
        let eff = cap[1].trim().to_string();
        if !eff.is_empty() {
            assets.particle_effects.insert(eff);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vmf() {
        let vmf = r#"
            world {
                "skyname" "sky_dust"
                "detailvbsp" "detail_custom.vbsp"
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
            entity {
                "classname" "info_particle_system"
                "effect_name" "smoke_campfireflame"
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
        assert!(assets.detail_vbsps.contains("detail_custom.vbsp"));
        assert!(assets.particle_effects.contains("smoke_campfireflame"));
    }
}
