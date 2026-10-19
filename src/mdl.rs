use crate::utils::normalize_source_path;

pub const MODEL_VARIANTS: [&str; 8] = [
    ".mdl",
    ".vvd",
    ".phy",
    ".dx90.vtx",
    ".dx80.vtx",
    ".sw.vtx",
    ".ani",
    ".xbox.vtx",
];

pub struct MdlInfo {
    pub textures: Vec<String>,
    pub texture_dirs: Vec<String>,
}

fn read_null_terminated_string(data: &[u8], offset: usize) -> Option<String> {
    if offset >= data.len() {
        return None;
    }
    let end = data[offset..].iter().position(|&b| b == 0)?;
    String::from_utf8(data[offset..offset + end].to_vec()).ok()
}

pub fn parse_mdl_textures(data: &[u8]) -> Option<MdlInfo> {
    if data.len() < 220 {
        return None;
    }

    // Magic IDST check
    if &data[0..4] != b"IDST" {
        return None;
    }

    let texture_count = i32::from_le_bytes(data[204..208].try_into().ok()?) as usize;
    let texture_offset = i32::from_le_bytes(data[208..212].try_into().ok()?) as usize;
    let texturedir_count = i32::from_le_bytes(data[212..216].try_into().ok()?) as usize;
    let texturedir_offset = i32::from_le_bytes(data[216..220].try_into().ok()?) as usize;

    let mut textures = Vec::new();
    for i in 0..texture_count {
        let entry_pos = texture_offset + i * 64;
        if entry_pos + 4 > data.len() {
            break;
        }
        let name_offset =
            i32::from_le_bytes(data[entry_pos..entry_pos + 4].try_into().ok()?) as isize;
        let str_pos = (entry_pos as isize + name_offset) as usize;
        if let Some(tex) = read_null_terminated_string(data, str_pos) {
            textures.push(normalize_source_path(&tex).to_lowercase());
        }
    }

    let mut texture_dirs = Vec::new();
    for i in 0..texturedir_count {
        let entry_pos = texturedir_offset + i * 4;
        if entry_pos + 4 > data.len() {
            break;
        }
        let name_offset =
            i32::from_le_bytes(data[entry_pos..entry_pos + 4].try_into().ok()?) as usize;
        // In Source Engine, cdtexture offsets in stringIndex can be direct file offsets
        if let Some(dir) = read_null_terminated_string(data, name_offset) {
            let norm = normalize_source_path(&dir)
                .trim_end_matches('/')
                .to_lowercase();
            texture_dirs.push(norm);
        } else if let Some(dir) = read_null_terminated_string(data, entry_pos + name_offset) {
            let norm = normalize_source_path(&dir)
                .trim_end_matches('/')
                .to_lowercase();
            texture_dirs.push(norm);
        }
    }

    Some(MdlInfo {
        textures,
        texture_dirs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_parse_real_mdl() {
        let test_path = r#"E:\SteamLibrary\steamapps\common\GarrysMod\bin\win64\hammerplusplus\models\editor\spot_color.mdl"#;
        if let Ok(bytes) = fs::read(test_path) {
            let info = parse_mdl_textures(&bytes).expect("Should parse MDL");
            assert_eq!(info.textures, vec!["spot_sheet_color"]);
            assert_eq!(info.texture_dirs, vec!["models/editor"]);
        }
    }
}
