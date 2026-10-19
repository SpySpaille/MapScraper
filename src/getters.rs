use std::collections::HashSet;

pub const TEXTURE_GETTERS_RAW: &str = include_str!("../Getters/TextureGetter.txt");
pub const MATERIAL_GETTERS_RAW: &str = include_str!("../Getters/MaterialGetter.txt");
pub const SOUND_GETTERS_RAW: &str = include_str!("../Getters/SoundGetter.txt");

pub struct Getters {
    pub texture_getters: HashSet<String>,
    pub material_getters: HashSet<String>,
    pub sound_getters: HashSet<String>,
}

impl Getters {
    pub fn load() -> Self {
        let parse_lines = |raw: &str| -> HashSet<String> {
            raw.lines()
                .map(|l| l.trim().to_lowercase())
                .filter(|l| !l.is_empty() && !l.starts_with("//"))
                .map(|l| l.trim_start_matches('$').trim_start_matches('%').to_string())
                .collect()
        };

        Self {
            texture_getters: parse_lines(TEXTURE_GETTERS_RAW),
            material_getters: parse_lines(MATERIAL_GETTERS_RAW),
            sound_getters: parse_lines(SOUND_GETTERS_RAW),
        }
    }
}
