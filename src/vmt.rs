use crate::keyvalues::{parse_keyvalues, KVValue};
use crate::utils::{clean_quotes, normalize_source_path};
use std::collections::HashMap;

pub fn parse_vmt(vmt_content: &str) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();

    if let Ok(doc) = parse_keyvalues(vmt_content) {
        for (root_key, root_val) in doc.root_entries {
            if root_key.to_lowercase() == "include" {
                if let KVValue::String(s) = root_val {
                    map.entry("include".to_string())
                        .or_default()
                        .push(clean_quotes(&s));
                }
            } else {
                traverse_kv_value(&root_val, &mut map);
            }
        }
    } else {
        // Fallback regex in case of non-standard file formatting
        let cleaned = vmt_content.replace('\r', "");
        let kv_regex = regex::Regex::new(
            r#"(?:"([^"]+)"|(\$?[%a-zA-Z0-9_\-\./]+))\s+(?:"([^"]+)"|([^\s{}"]+))"#,
        )
        .unwrap();
        for cap in kv_regex.captures_iter(&cleaned) {
            let raw_key = cap
                .get(1)
                .or_else(|| cap.get(2))
                .map(|m| m.as_str())
                .unwrap_or("");
            let raw_val = cap
                .get(3)
                .or_else(|| cap.get(4))
                .map(|m| m.as_str())
                .unwrap_or("");
            if raw_key.is_empty() || raw_val.is_empty() || raw_key.starts_with("//") {
                continue;
            }
            let mut key = clean_quotes(raw_key).to_lowercase();
            if key.starts_with('$') || key.starts_with('%') {
                key = key[1..].to_string();
            }
            let value = clean_quotes(raw_val);
            if !key.is_empty() && !value.is_empty() {
                map.entry(key).or_default().push(value);
            }
        }
    }

    map
}

fn traverse_kv_value(val: &KVValue, map: &mut HashMap<String, Vec<String>>) {
    if let KVValue::Block(entries) = val {
        for (raw_key, child_val) in entries {
            let mut key = clean_quotes(raw_key).to_lowercase();
            if key.starts_with('$') || key.starts_with('%') {
                key = key[1..].to_string();
            }
            match child_val {
                KVValue::String(s) => {
                    let value = clean_quotes(s);
                    if !key.is_empty() && !value.is_empty() {
                        map.entry(key).or_default().push(value);
                    }
                }
                KVValue::Block(_) => {
                    if key == "include" {
                        if let Some(inc_path) = child_val.get_string("include") {
                            map.entry("include".to_string())
                                .or_default()
                                .push(clean_quotes(inc_path));
                        }
                    }
                    traverse_kv_value(child_val, map);
                }
            }
        }
    }
}

pub fn normalize_texture_path(texture_path: &str) -> String {
    let mut p = normalize_source_path(texture_path);
    if p.to_lowercase().starts_with("materials/") {
        p = p["materials/".len()..].to_string();
    }
    if p.to_lowercase().ends_with(".vtf") || p.to_lowercase().ends_with(".vmt") {
        p = p[..p.len() - 4].to_string();
    }
    p
}

/// Resolves the master base material for compiled Source Engine patch materials:
/// - WVT displacement patches: `maps/<mapname>/<subpath>_wvt_patch` -> `<subpath>`
/// - Standalone WVT patches: `<subpath>_wvt_patch` -> `<subpath>`
/// - Cubemap envmap reflections: `maps/<mapname>/<subpath>_<x>_<y>_<z>` -> `<subpath>`
/// - Water depth patches: `maps/<mapname>/<subpath>_depth_<val>` -> `<subpath>`
/// - General map prefix: `maps/<mapname>/<subpath>` -> `<subpath>`
pub fn resolve_base_material(material_path: &str) -> Option<String> {
    let mut clean = normalize_texture_path(material_path);
    let lower = clean.to_lowercase();
    let mut changed = false;

    // 1. Strip maps/<mapname>/ prefix if present
    if lower.starts_with("maps/") {
        let rest = &clean[5..]; // after "maps/"
        if let Some(slash_pos) = rest.find('/') {
            clean = rest[slash_pos + 1..].to_string();
            changed = true;
        }
    }

    let mut current_lower = clean.to_lowercase();

    // 2. Strip _wvt_patch suffix
    if current_lower.ends_with("_wvt_patch") {
        clean = clean[..clean.len() - "_wvt_patch".len()].to_string();
        current_lower = clean.to_lowercase();
        changed = true;
    }

    // 3. Strip water depth: _depth_\d+
    if let Some(pos) = current_lower.rfind("_depth_") {
        let suffix = &current_lower[pos + 7..];
        if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) {
            clean = clean[..pos].to_string();
            current_lower = clean.to_lowercase();
            changed = true;
        }
    }

    // 4. Strip cubemap coordinates: _-?\d+_-?\d+_-?\d+
    // E.g. _3512_-841_-2924
    if let Some(last_underscore) = current_lower.rfind('_') {
        let z_part = &current_lower[last_underscore + 1..];
        let is_z = !z_part.is_empty()
            && (z_part.chars().all(|c| c.is_ascii_digit())
                || (z_part.starts_with('-') && z_part[1..].chars().all(|c| c.is_ascii_digit())));
        if is_z {
            if let Some(mid_underscore) = current_lower[..last_underscore].rfind('_') {
                let y_part = &current_lower[mid_underscore + 1..last_underscore];
                let is_y = !y_part.is_empty()
                    && (y_part.chars().all(|c| c.is_ascii_digit())
                        || (y_part.starts_with('-')
                            && y_part[1..].chars().all(|c| c.is_ascii_digit())));
                if is_y {
                    if let Some(first_underscore) = current_lower[..mid_underscore].rfind('_') {
                        let x_part = &current_lower[first_underscore + 1..mid_underscore];
                        let is_x = !x_part.is_empty()
                            && (x_part.chars().all(|c| c.is_ascii_digit())
                                || (x_part.starts_with('-')
                                    && x_part[1..].chars().all(|c| c.is_ascii_digit())));
                        if is_x {
                            clean = clean[..first_underscore].to_string();
                            changed = true;
                        }
                    }
                }
            }
        }
    }

    if changed && !clean.is_empty() {
        Some(clean)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vmt_basic() {
        let vmt = r#"
            // Header comment
            "LightmappedGeneric"
            {
                "$basetexture" "brick/wall01"
                "$bumpmap" "brick/wall01_normal.vtf"
                "%keywords" "c17"
            }
        "#;
        let map = parse_vmt(vmt);
        assert_eq!(map.get("basetexture").unwrap()[0], "brick/wall01");
        assert_eq!(map.get("bumpmap").unwrap()[0], "brick/wall01_normal.vtf");
        assert_eq!(map.get("keywords").unwrap()[0], "c17");
    }

    #[test]
    fn test_parse_vmt_crlf_and_comments() {
        let vmt = "\"WorldVertexTransition\"\r\n{\r\n    // Alpha 0: custom_ground01\r\n    // Alpha 1: custom_grass01\r\n\t\"$basetexture\" \"nature/dirtground002_dark\"\r\n    \"$bumpmap\" \"custom_env/ext_terrain/terrain_soil01_n\"\r\n    \"$surfaceprop\" \"dirt\"\r\n\r\n\r\n    \"$basetexture2\" \"custom_env/ext_terrain/terrain_grass01\"\r\n    \"$bumpmap2\" \"custom_env/ext_terrain/terrain_grass01_n\"\r\n    \"$surfaceprop2\" \"grass\"\r\n\r\n    $blendmodulatetexture \"dbg/blend_texture_01\"\r\n\r\n    // Linear vertex-alpha blend\r\n    \"$detail\" \"custom_env/ext_terrain/detail_grass_v2\"\r\n}\r\n";
        let map = parse_vmt(vmt);
        assert_eq!(
            map.get("basetexture").unwrap()[0],
            "nature/dirtground002_dark"
        );
        assert_eq!(
            map.get("bumpmap").unwrap()[0],
            "custom_env/ext_terrain/terrain_soil01_n"
        );
        assert_eq!(
            map.get("basetexture2").unwrap()[0],
            "custom_env/ext_terrain/terrain_grass01"
        );
        assert_eq!(
            map.get("bumpmap2").unwrap()[0],
            "custom_env/ext_terrain/terrain_grass01_n"
        );
        assert_eq!(
            map.get("blendmodulatetexture").unwrap()[0],
            "dbg/blend_texture_01"
        );
        assert_eq!(
            map.get("detail").unwrap()[0],
            "custom_env/ext_terrain/detail_grass_v2"
        );
    }

    #[test]
    fn test_parse_vmt_patch() {
        let patch = r#"
            "Patch"
            {
                "include" "materials/models/props/crate.vmt"
                "insert"
                {
                    "$detail" "detail/wood"
                }
            }
        "#;
        let map = parse_vmt(patch);
        assert_eq!(
            map.get("include").unwrap()[0],
            "materials/models/props/crate.vmt"
        );
        assert_eq!(map.get("detail").unwrap()[0], "detail/wood");
    }

    #[test]
    fn test_normalize_texture_path() {
        assert_eq!(
            normalize_texture_path("materials/models/crate.vtf"),
            "models/crate"
        );
        assert_eq!(normalize_texture_path("brick/wall01.vmt"), "brick/wall01");
        assert_eq!(
            normalize_texture_path("\"concrete/floor\""),
            "concrete/floor"
        );
    }

    #[test]
    fn test_resolve_base_material() {
        assert_eq!(
            resolve_base_material("maps/test_map_v1/terrain/blend_ground01_to_gravel_wvt_patch"),
            Some("terrain/blend_ground01_to_gravel".to_string())
        );
        assert_eq!(
            resolve_base_material("maps/test_map_v1/architecture/marble_black_01_3512_-841_-2924"),
            Some("architecture/marble_black_01".to_string())
        );
        assert_eq!(
            resolve_base_material("nature/blenddirtlawn01_wvt_patch"),
            Some("nature/blenddirtlawn01".to_string())
        );
        assert_eq!(
            resolve_base_material("dev/water_depth_128"),
            Some("dev/water".to_string())
        );
        assert_eq!(
            resolve_base_material("brick/wall01"),
            None
        );
    }
}
