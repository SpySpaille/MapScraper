use std::fmt;
use std::path::{Component, Path};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VirtualAssetPath(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathError {
    Empty,
    AbsoluteNotAllowed(String),
    ParentDirTraversal(String),
    InvalidCharacters(String),
}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathError::Empty => write!(f, "Virtual asset path cannot be empty"),
            PathError::AbsoluteNotAllowed(p) => {
                write!(f, "Absolute path or drive prefix not allowed: {}", p)
            }
            PathError::ParentDirTraversal(p) => {
                write!(f, "Parent directory traversal '..' not allowed: {}", p)
            }
            PathError::InvalidCharacters(p) => write!(
                f,
                "Path contains invalid control characters or null bytes: {}",
                p
            ),
        }
    }
}

impl std::error::Error for PathError {}

impl VirtualAssetPath {
    pub fn parse(raw: &str) -> Result<Self, PathError> {
        let trimmed = raw.trim().trim_matches('"').trim_matches('\'');
        if trimmed.is_empty() {
            return Err(PathError::Empty);
        }

        if trimmed.contains('\0') {
            return Err(PathError::InvalidCharacters(trimmed.to_string()));
        }

        // Check for Windows drive prefixes (e.g. C:, D:)
        if trimmed.len() >= 2 && trimmed.as_bytes()[1] == b':' {
            return Err(PathError::AbsoluteNotAllowed(trimmed.to_string()));
        }

        // Convert backslashes to forward slashes
        let unified = trimmed.replace('\\', "/");

        // Disallow leading slashes
        if unified.starts_with('/') {
            return Err(PathError::AbsoluteNotAllowed(unified));
        }

        // Parse path components to reject any '..' traversal
        let path = Path::new(&unified);
        for comp in path.components() {
            match comp {
                Component::ParentDir => {
                    return Err(PathError::ParentDirTraversal(unified));
                }
                Component::Prefix(_) | Component::RootDir => {
                    return Err(PathError::AbsoluteNotAllowed(unified));
                }
                _ => {}
            }
        }

        // Filter out redundant empty segments or '.'
        let segments: Vec<&str> = unified
            .split('/')
            .filter(|s| !s.is_empty() && *s != ".")
            .collect();

        if segments.is_empty() {
            return Err(PathError::Empty);
        }

        Ok(Self(segments.join("/")))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn to_lowercase_path(&self) -> Self {
        Self(self.0.to_lowercase())
    }

    pub fn file_name(&self) -> Option<&str> {
        self.0.rsplit('/').next()
    }

    pub fn extension(&self) -> Option<&str> {
        self.file_name()
            .and_then(|f| f.rsplit('.').next().filter(|ext| *ext != f))
    }
}

impl fmt::Display for VirtualAssetPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl serde::Serialize for VirtualAssetPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> serde::Deserialize<'de> for VirtualAssetPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        VirtualAssetPath::parse(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_virtual_paths() {
        let p = VirtualAssetPath::parse("materials\\models\\crate.vtf").unwrap();
        assert_eq!(p.as_str(), "materials/models/crate.vtf");
        assert_eq!(p.extension(), Some("vtf"));

        let p2 = VirtualAssetPath::parse("./sound/ambient/creak.wav").unwrap();
        assert_eq!(p2.as_str(), "sound/ambient/creak.wav");
    }

    #[test]
    fn test_reject_traversal() {
        assert!(VirtualAssetPath::parse("../../secret.txt").is_err());
        assert!(VirtualAssetPath::parse("models/../../secret.txt").is_err());
    }

    #[test]
    fn test_reject_absolute() {
        assert!(VirtualAssetPath::parse("C:/Windows/system32/calc.exe").is_err());
        assert!(VirtualAssetPath::parse("/etc/passwd").is_err());
        assert!(VirtualAssetPath::parse("\\\\server\\share\\asset").is_err());
    }
}
