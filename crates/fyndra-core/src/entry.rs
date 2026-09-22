use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const FLAG_IS_DIR: u8 = 0x01;
pub const FLAG_IS_SYMLINK: u8 = 0x02;
pub const FLAG_IS_REMOVABLE: u8 = 0x04;

/// Rich, user-facing representation of a file entry.
/// Used for UI display, CLI outputs, and external integrations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileEntry {
    pub name: String,
    pub name_lower: String,
    pub parent: PathBuf,
    pub size: u64,
    pub is_dir: bool,
    pub modified: u64, // Unix timestamp in seconds
}

impl FileEntry {
    pub fn new(path: &Path, metadata: Option<&std::fs::Metadata>) -> Option<Self> {
        let name = path.file_name()?.to_string_lossy().to_string();
        let name_lower = name.to_lowercase();
        let parent = path.parent().unwrap_or(Path::new("/")).to_path_buf();

        let (size, is_dir, modified) = if let Some(meta) = metadata {
            let size = meta.len();
            let is_dir = meta.is_dir();
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            (size, is_dir, modified)
        } else if let Ok(meta) = std::fs::symlink_metadata(path) {
            let size = meta.len();
            let is_dir = meta.is_dir();
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            (size, is_dir, modified)
        } else {
            (0, false, 0)
        };

        Some(Self { name, name_lower, parent, size, is_dir, modified })
    }

    #[inline]
    pub fn full_path(&self) -> PathBuf {
        self.parent.join(&self.name)
    }

    #[inline]
    pub fn extension(&self) -> Option<&str> {
        let name = &self.name;
        let dot_pos = name.rfind('.')?;
        if dot_pos == 0 || dot_pos == name.len() - 1 {
            None
        } else {
            Some(&name[dot_pos + 1..])
        }
    }
}

/// Highly compact 32-byte in-memory representation.
/// Eliminates redundant parent PathBufs and heap bloat across 10M-50M+ files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompactEntry {
    pub dir_id: u32,
    pub name: Box<str>,
    pub size: u64,
    pub modified: u32,
    pub flags: u8,
}

impl CompactEntry {
    #[inline]
    pub fn is_dir(&self) -> bool {
        (self.flags & FLAG_IS_DIR) != 0
    }

    #[inline]
    pub fn is_symlink(&self) -> bool {
        (self.flags & FLAG_IS_SYMLINK) != 0
    }

    #[inline]
    pub fn is_removable(&self) -> bool {
        (self.flags & FLAG_IS_REMOVABLE) != 0
    }

    #[inline]
    pub fn extension(&self) -> Option<&str> {
        let name = &self.name;
        let dot_pos = name.rfind('.')?;
        if dot_pos == 0 || dot_pos == name.len() - 1 {
            None
        } else {
            Some(&name[dot_pos + 1..])
        }
    }

    #[inline]
    pub fn to_file_entry(&self, dirs: &DirectoryStore) -> FileEntry {
        let parent = dirs.get_path(self.dir_id).unwrap_or_else(|| PathBuf::from("/"));
        let name_str = self.name.to_string();
        let name_lower = name_str.to_lowercase();
        FileEntry {
            name: name_str,
            name_lower,
            parent,
            size: self.size,
            is_dir: self.is_dir(),
            modified: self.modified as u64,
        }
    }
}

/// Centralized directory repository.
/// 50M files typically reside in ~500k-2M unique directories. Storing them once
/// saves gigabytes of redundant PathBuf allocations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DirectoryStore {
    dirs: Vec<PathBuf>,
    #[serde(skip)]
    lookup: HashMap<PathBuf, u32>,
}

impl DirectoryStore {
    pub fn new() -> Self {
        Self { dirs: Vec::with_capacity(1024), lookup: HashMap::with_capacity(1024) }
    }

    /// Rebuilds lookup map after deserialization
    pub fn rebuild_lookup(&mut self) {
        self.lookup.clear();
        for (idx, dir) in self.dirs.iter().enumerate() {
            self.lookup.insert(dir.clone(), idx as u32);
        }
    }

    pub fn get_or_insert(&mut self, path: &Path) -> u32 {
        if let Some(&id) = self.lookup.get(path) {
            return id;
        }

        let id = self.dirs.len() as u32;
        let pbuf = path.to_path_buf();
        self.lookup.insert(pbuf.clone(), id);
        self.dirs.push(pbuf);
        id
    }

    #[inline]
    pub fn find_id(&self, path: &Path) -> Option<u32> {
        self.lookup.get(path).copied()
    }

    #[inline]
    pub fn get_path(&self, id: u32) -> Option<PathBuf> {
        self.dirs.get(id as usize).cloned()
    }

    #[inline]
    pub fn get_path_ref(&self, id: u32) -> Option<&Path> {
        self.dirs.get(id as usize).map(|p| p.as_path())
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.dirs.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.dirs.is_empty()
    }

    pub fn dirs(&self) -> &[PathBuf] {
        &self.dirs
    }
}
