use std::fs;
use std::path::Path;

pub fn clean_quotes(s: &str) -> String {
    s.trim().trim_matches(|c| c == '"' || c == '\'').to_string()
}

pub fn normalize_source_path(p: &str) -> String {
    let cleaned = clean_quotes(p);
    cleaned.replace('\\', "/").trim_start_matches('/').to_string()
}

pub fn clean_sound_path(s: &str) -> String {
    let mut norm = normalize_source_path(s);
    norm = norm.trim_start_matches(|c| matches!(c, '*' | ')' | '^' | '#' | '~' | '>' | '<')).trim().to_string();
    if norm.to_lowercase().starts_with("sound/") {
        norm = norm["sound/".len()..].to_string();
    }
    norm
}

pub fn safe_copy_file<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) -> std::io::Result<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();
    if !src.exists() {
        return Ok(());
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(src, dst)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_source_path() {
        assert_eq!(normalize_source_path("\"materials\\models\\crate.vtf\""), "materials/models/crate.vtf");
        assert_eq!(normalize_source_path("/sound/weapons/c4.wav"), "sound/weapons/c4.wav");
    }

    #[test]
    fn test_clean_sound_path() {
        assert_eq!(clean_sound_path(")weapons/c4/c4_beep1.wav"), "weapons/c4/c4_beep1.wav");
        assert_eq!(clean_sound_path("^ambient/wind.wav"), "ambient/wind.wav");
        assert_eq!(clean_sound_path("*ambient/generator.wav"), "ambient/generator.wav");
        assert_eq!(clean_sound_path("sound/doors/door1.wav"), "doors/door1.wav");
    }
}
