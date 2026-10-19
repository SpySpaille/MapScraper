use std::collections::HashMap;
use crate::utils::{clean_quotes, normalize_source_path};

pub fn strip_vmt_comments(content: &str) -> String {
    // 1. Remove block comments /* ... */
    let re_block = regex::Regex::new(r"(?s)/\*.*?\*/").unwrap();
    let no_block = re_block.replace_all(content, "");

    // 2. Remove line comments // ...
    let re_line = regex::Regex::new(r"//.*$").unwrap();
    re_line.replace_all(&no_block, "").to_string()
}

pub fn parse_vmt(vmt_content: &str) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    let cleaned = strip_vmt_comments(vmt_content);

    // Regex for matching key value pairs while ignoring block braces
    let kv_regex = regex::Regex::new(r#"(?:"([^"]+)"|(\$?[%a-zA-Z0-9_\-\./]+))\s+(?:"([^"]+)"|([^\s{}"]+))"#).unwrap();

    for cap in kv_regex.captures_iter(&cleaned) {
        let raw_key = cap.get(1).or_else(|| cap.get(2)).map(|m| m.as_str()).unwrap_or("");
        let raw_val = cap.get(3).or_else(|| cap.get(4)).map(|m| m.as_str()).unwrap_or("");

        if raw_key.is_empty() || raw_val.is_empty() {
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

    // Include directive (e.g. Patch shader: include "materials/models/...")
    let inc_regex = regex::Regex::new(r#"(?i)["']?include["']?\s+["']([^"']+)["']"#).unwrap();
    for cap in inc_regex.captures_iter(&cleaned) {
        let inc_val = clean_quotes(&cap[1]);
        if !inc_val.is_empty() {
            let list = map.entry("include".to_string()).or_default();
            if !list.contains(&inc_val) {
                list.push(inc_val);
            }
        }
    }

    map
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
        assert_eq!(map.get("include").unwrap()[0], "materials/models/props/crate.vmt");
        assert_eq!(map.get("detail").unwrap()[0], "detail/wood");
    }

    #[test]
    fn test_normalize_texture_path() {
        assert_eq!(normalize_texture_path("materials/models/crate.vtf"), "models/crate");
        assert_eq!(normalize_texture_path("brick/wall01.vmt"), "brick/wall01");
        assert_eq!(normalize_texture_path("\"concrete/floor\""), "concrete/floor");
    }
}
