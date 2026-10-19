use crate::paths::VirtualAssetPath;
use std::fs;
use std::io;
use std::path::Path;

pub fn clean_quotes(s: &str) -> String {
    s.trim().trim_matches(|c| c == '"' || c == '\'').to_string()
}

pub fn normalize_source_path(p: &str) -> String {
    let cleaned = clean_quotes(p);
    match VirtualAssetPath::parse(&cleaned) {
        Ok(v) => v.as_str().to_string(),
        Err(_) => {
            // Fallback for non-compliant or empty paths, removing traversal sequences
            let sanitized = cleaned.replace('\\', "/").replace("..", "");
            sanitized.trim_matches('/').to_string()
        }
    }
}

pub fn clean_sound_path(s: &str) -> String {
    let mut norm = normalize_source_path(s);
    norm = norm
        .trim_start_matches(['*', ')', '^', '#', '~', '>', '<'])
        .trim()
        .to_string();
    if norm.to_lowercase().starts_with("sound/") {
        norm = norm["sound/".len()..].to_string();
    }
    norm
}

pub fn safe_copy_file<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) -> io::Result<u64> {
    let src = src.as_ref();
    let dst = dst.as_ref();
    if !src.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Source file does not exist: {}", src.display()),
        ));
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(src, dst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_source_path() {
        assert_eq!(
            normalize_source_path("\"materials\\models\\crate.vtf\""),
            "materials/models/crate.vtf"
        );
        assert_eq!(
            normalize_source_path("/sound/weapons/c4.wav"),
            "sound/weapons/c4.wav"
        );
        assert_eq!(normalize_source_path("../../secret.txt"), "secret.txt");
    }

    #[test]
    fn test_clean_sound_path() {
        assert_eq!(
            clean_sound_path(")weapons/c4/c4_beep1.wav"),
            "weapons/c4/c4_beep1.wav"
        );
        assert_eq!(clean_sound_path("^ambient/wind.wav"), "ambient/wind.wav");
        assert_eq!(
            clean_sound_path("*ambient/generator.wav"),
            "ambient/generator.wav"
        );
        assert_eq!(clean_sound_path("sound/doors/door1.wav"), "doors/door1.wav");
    }

    #[test]
    fn test_safe_copy_file_missing_source() {
        let missing = Path::new("tests/fixtures/non_existent_123.txt");
        let dst = Path::new("tests/fixtures/dst_123.txt");
        assert!(safe_copy_file(missing, dst).is_err());
    }
}
