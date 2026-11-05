//! The diff of one file as plain data: its hunks and lines, the two sides
//! of an image, and what it was taken against. Also the patches that take
//! one hunk or one line back out of the working tree.
//!
//! It has no engine and no git. The diff view of `hilen` draws these
//! types, and a program with no window, like a daemon that runs git,
//! builds them and sends them as JSON. Why it is a repo of its own is in
//! the README.

mod patch;

use std::hash::{DefaultHasher, Hash, Hasher};

use serde::{Deserialize, Serialize};

pub use crate::patch::{hunk_patch, line_patch};

/// 1 line of a hunk. `kind` is `add`, `rem` or `ctx`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DiffLine {
    pub kind:   String,
    pub text:   String,
    pub old_no: Option<u32>,
    pub new_no: Option<u32>,
}

/// 1 block of changed lines, the `@@ -10,3 +10,4 @@` of a patch.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub lines:     Vec<DiffLine>,
}

/// The bytes of one side of an image diff. A view keeps a loaded image
/// by its name, so the name holds a hash of the bytes and a changed file
/// gets a new one.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Blob {
    pub name:  String,
    #[serde(with = "base64_bytes")]
    pub bytes: Vec<u8>,
}

impl Blob {
    pub fn new(file: &str, bytes: Vec<u8>) -> Self {
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        Self {
            name: format!("{file}#{:016x}", hasher.finish()),
            bytes,
        }
    }
}

/// Bytes go over JSON as base64 text, an array of numbers is 4 times
/// bigger.
mod base64_bytes {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        let text = String::deserialize(deserializer)?;
        STANDARD.decode(text).map_err(serde::de::Error::custom)
    }
}

/// What a changed file shows.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FileDiffBody {
    Text { hunks: Vec<DiffHunk> },
    Binary,
    Image { old: Option<Blob>, new: Option<Blob> },
    Empty,
}

/// The diff of 1 file.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct FileDiff {
    pub path:       String,
    pub old_path:   Option<String>,
    pub is_new:     bool,
    pub is_deleted: bool,
    pub body:       FileDiffBody,
}

/// What a file diff is taken against. A view keys its cache by it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DiffTarget {
    /// The working tree and the index against HEAD.
    WorkingTree,
    /// One commit against its first parent.
    Commit { sha: String },
    /// A stash entry against its base, by stash index.
    Stash { index: usize },
    /// Everything from one commit up to another, or up to the working
    /// tree when `to` is None. The branch diff of the compare mode.
    Range { from: String, to: Option<String> },
}

impl DiffTarget {
    pub fn commit(sha: impl Into<String>) -> Self {
        Self::Commit { sha: sha.into() }
    }

    /// True when the diff shows the working tree, so it changes under
    /// the view while the repository is edited.
    pub fn against_working_tree(&self) -> bool {
        matches!(self, Self::WorkingTree | Self::Range { to: None, .. })
    }

    /// A key for a diff that can never change, so it can be cached.
    pub fn cache_key(&self, file: &str) -> Option<String> {
        match self {
            Self::Commit { sha } => Some(format!("{sha}:{file}")),
            Self::Range { from, to: Some(to) } => Some(format!("{from}..{to}:{file}")),
            Self::WorkingTree | Self::Stash { .. } | Self::Range { to: None, .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Blob, DiffTarget, FileDiffBody};

    #[test]
    fn an_image_side_goes_over_json_as_base64() {
        let body = FileDiffBody::Image {
            old: None,
            new: Some(Blob::new("a.png", vec![0, 1, 2, 255])),
        };
        let json = serde_json::to_string(&body).unwrap();
        assert!(json.contains(r#""bytes":"AAEC/w==""#), "{json}");
        let back: FileDiffBody = serde_json::from_str(&json).unwrap();
        assert_eq!(back, body);
    }

    #[test]
    fn a_changed_image_gets_a_new_name() {
        let before = Blob::new("a.png", vec![1]);
        let after = Blob::new("a.png", vec![2]);
        assert_ne!(before.name, after.name);
        assert_eq!(before.name, Blob::new("a.png", vec![1]).name);
    }

    #[test]
    fn only_a_diff_that_cannot_change_has_a_cache_key() {
        assert_eq!(
            DiffTarget::commit("abc").cache_key("a.rs"),
            Some("abc:a.rs".to_string())
        );
        assert_eq!(DiffTarget::WorkingTree.cache_key("a.rs"), None);
        let open_range = DiffTarget::Range {
            from: "abc".to_string(),
            to:   None,
        };
        assert!(open_range.against_working_tree());
        assert_eq!(open_range.cache_key("a.rs"), None);
    }
}
