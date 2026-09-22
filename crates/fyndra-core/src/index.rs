use crate::crawler::{CrawlStats, Crawler, CrawlerConfig};
use crate::entry::{
    CompactEntry, DirectoryStore, FileEntry, FLAG_IS_DIR, FLAG_IS_REMOVABLE, FLAG_IS_SYMLINK,
};
use crate::search::{SearchEngine, SearchQuery, SearchResult};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::RwLock;

const CACHE_MAGIC: &[u8; 4] = b"EVTH";
const CACHE_VERSION: u32 = 1;

#[derive(Clone, Default)]
pub struct IndexData {
    pub entries: Vec<CompactEntry>,
    pub dirs: DirectoryStore,
    /// O(1) lookup for upsert/remove: (dir_id, name) -> index in entries.
    /// Not serialized — rebuilt after load (saves cache size).
    pub entry_index: HashMap<(u32, String), usize>,
}

impl IndexData {
    pub fn new() -> Self {
        Self { entries: Vec::new(), dirs: DirectoryStore::new(), entry_index: HashMap::new() }
    }

    /// Rebuilds entry_index from entries — call after bulk load or deserialization.
    pub fn rebuild_entry_index(&mut self) {
        self.entry_index.clear();
        // Reserve to avoid rehashing at 50M scale
        self.entry_index.reserve(self.entries.len());
        for (idx, e) in self.entries.iter().enumerate() {
            self.entry_index.insert((e.dir_id, e.name.to_string()), idx);
        }
    }

    /// Approximate RAM usage in bytes
    pub fn memory_usage_bytes(&self) -> usize {
        let entries_mem = self.entries.capacity() * std::mem::size_of::<CompactEntry>()
            + self.entries.iter().map(|e| e.name.len()).sum::<usize>();
        let dirs_mem = self.dirs.dirs().iter().map(|d| d.as_os_str().len() + 32).sum::<usize>();
        let index_mem = self.entry_index.capacity()
            * (std::mem::size_of::<(u32, String)>() + std::mem::size_of::<usize>() + 32);
        entries_mem + dirs_mem + index_mem
    }
}

#[derive(Clone)]
pub struct Database {
    data: Arc<RwLock<IndexData>>,
    config: Arc<CrawlerConfig>,
}

impl Default for Database {
    fn default() -> Self {
        Self::new(CrawlerConfig::default())
    }
}

impl Database {
    pub fn new(config: CrawlerConfig) -> Self {
        Self { data: Arc::new(RwLock::new(IndexData::new())), config: Arc::new(config) }
    }

    /// Indexes the specified root directories
    pub fn index_roots(&self, roots: &[PathBuf]) -> CrawlStats {
        let crawler = Crawler::new(CrawlerConfig {
            excluded_paths: self.config.excluded_paths.clone(),
            follow_symlinks: self.config.follow_symlinks,
            max_depth: self.config.max_depth,
            index_removable_drives: self.config.index_removable_drives,
        });

        let (crawled, dir_store, stats) = crawler.crawl_roots_compact(roots);
        {
            let mut write_guard = self.data.write();
            write_guard.entries = crawled;
            write_guard.dirs = dir_store;
            write_guard.rebuild_entry_index();
        }
        stats
    }

    /// Performs ultra-fast search against in-memory compact index
    pub fn search(&self, query: &SearchQuery) -> SearchResult {
        let read_guard = self.data.read();
        SearchEngine::execute_compact(&read_guard.entries, &read_guard.dirs, query)
    }

    /// Total entries currently indexed
    pub fn len(&self) -> usize {
        self.data.read().entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Total unique directories indexed
    pub fn total_directories(&self) -> usize {
        self.data.read().dirs.len()
    }

    /// Accurate RAM consumption of the index in bytes
    pub fn memory_usage_bytes(&self) -> usize {
        self.data.read().memory_usage_bytes()
    }

    /// Inserts or updates an entry for a given path
    pub fn upsert_path(&self, path: &Path) {
        let parent = path.parent().unwrap_or(Path::new("/"));
        let name = match path.file_name() {
            Some(n) => n.to_string_lossy().into_owned().into_boxed_str(),
            None => return,
        };

        let (size, modified, flags) = if let Ok(meta) = fs::symlink_metadata(path) {
            let is_symlink = meta.file_type().is_symlink();
            let is_dir = meta.is_dir();
            let size = if is_dir { 0 } else { meta.len() };
            let mod_time = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as u32)
                .unwrap_or(0);

            let mut f = 0u8;
            if is_dir {
                f |= FLAG_IS_DIR;
            }
            if is_symlink {
                f |= FLAG_IS_SYMLINK;
            }
            if path.starts_with("/media")
                || path.starts_with("/run/media")
                || path.starts_with("/mnt")
            {
                f |= FLAG_IS_REMOVABLE;
            }
            (size, mod_time, f)
        } else {
            return;
        };

        let mut write_guard = self.data.write();
        let dir_id = write_guard.dirs.get_or_insert(parent);
        let key = (dir_id, name.to_string());

        if let Some(&idx) = write_guard.entry_index.get(&key) {
            if let Some(existing) = write_guard.entries.get_mut(idx) {
                existing.size = size;
                existing.modified = modified;
                existing.flags = flags;
            }
        } else {
            let idx = write_guard.entries.len();
            write_guard.entries.push(CompactEntry { dir_id, name, size, modified, flags });
            write_guard.entry_index.insert(key, idx);
        }
    }

    /// Batch inserts or updates entries with a single write lock acquisition
    pub fn batch_upsert(&self, paths: &[PathBuf]) {
        if paths.is_empty() {
            return;
        }

        let mut write_guard = self.data.write();
        for path in paths {
            let parent = path.parent().unwrap_or(Path::new("/"));
            let name = match path.file_name() {
                Some(n) => n.to_string_lossy().into_owned().into_boxed_str(),
                None => continue,
            };

            let (size, modified, flags) = if let Ok(meta) = fs::symlink_metadata(path) {
                let is_symlink = meta.file_type().is_symlink();
                let is_dir = meta.is_dir();
                let size = if is_dir { 0 } else { meta.len() };
                let mod_time = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as u32)
                    .unwrap_or(0);

                let mut f = 0u8;
                if is_dir {
                    f |= FLAG_IS_DIR;
                }
                if is_symlink {
                    f |= FLAG_IS_SYMLINK;
                }
                if path.starts_with("/media")
                    || path.starts_with("/run/media")
                    || path.starts_with("/mnt")
                {
                    f |= FLAG_IS_REMOVABLE;
                }
                (size, mod_time, f)
            } else {
                continue;
            };

            let dir_id = write_guard.dirs.get_or_insert(parent);
            let key = (dir_id, name.to_string());
            if let Some(&idx) = write_guard.entry_index.get(&key) {
                if let Some(existing) = write_guard.entries.get_mut(idx) {
                    existing.size = size;
                    existing.modified = modified;
                    existing.flags = flags;
                }
            } else {
                let idx = write_guard.entries.len();
                write_guard.entries.push(CompactEntry { dir_id, name, size, modified, flags });
                write_guard.entry_index.insert(key, idx);
            }
        }
    }

    /// Removes an entry matching a given path — rebuilds index after remove (rare, correctness over micro-opt)
    pub fn remove_path(&self, path: &Path) {
        let parent = path.parent().unwrap_or(Path::new("/"));
        let name = match path.file_name() {
            Some(n) => n.to_string_lossy(),
            None => return,
        };

        let mut write_guard = self.data.write();
        if let Some(dir_id) = write_guard.dirs.find_id(parent) {
            let key = (dir_id, name.to_string());
            if let Some(idx) = write_guard.entry_index.remove(&key) {
                // Swap-remove to keep O(1); update index for swapped entry if not last
                let last_idx = write_guard.entries.len() - 1;
                write_guard.entries.swap_remove(idx);
                if idx != last_idx {
                    if let Some(swapped) = write_guard.entries.get(idx) {
                        let swapped_key = (swapped.dir_id, swapped.name.to_string());
                        write_guard.entry_index.insert(swapped_key, idx);
                    }
                }
            }
        }
    }

    /// Batch removes entries with a single write lock acquisition
    pub fn batch_remove(&self, paths: &[PathBuf]) {
        if paths.is_empty() {
            return;
        }

        let mut write_guard = self.data.write();
        let mut removed = false;
        for path in paths {
            let parent = path.parent().unwrap_or(Path::new("/"));
            let name = match path.file_name() {
                Some(n) => n.to_string_lossy(),
                None => continue,
            };

            if let Some(dir_id) = write_guard.dirs.find_id(parent) {
                let key = (dir_id, name.to_string());
                if let Some(idx) = write_guard.entry_index.remove(&key) {
                    let last_idx = write_guard.entries.len() - 1;
                    write_guard.entries.swap_remove(idx);
                    if idx != last_idx {
                        if let Some(swapped) = write_guard.entries.get(idx) {
                            let swapped_key = (swapped.dir_id, swapped.name.to_string());
                            write_guard.entry_index.insert(swapped_key, idx);
                        }
                    }
                    removed = true;
                }
            }
        }
        // Rebuild to ensure consistency if multiple removes shifted indices — swap logic above keeps consistent, but rebuild is cheap for batch and guarantees correctness
        if removed && write_guard.entry_index.len() != write_guard.entries.len() {
            write_guard.rebuild_entry_index();
        }
    }

    /// Ultra-fast binary serialization cache
    pub fn save_cache(&self, cache_file: &Path) -> io::Result<()> {
        if let Some(parent) = cache_file.parent() {
            fs::create_dir_all(parent)?;
        }

        let read_guard = self.data.read();
        let file = File::create(cache_file)?;
        let mut writer = BufWriter::with_capacity(128 * 1024, file);

        // 1. Magic + Version
        writer.write_all(CACHE_MAGIC)?;
        writer.write_all(&CACHE_VERSION.to_le_bytes())?;

        // 2. Directories
        let dirs = read_guard.dirs.dirs();
        writer.write_all(&(dirs.len() as u32).to_le_bytes())?;
        for dir in dirs {
            let s = dir.to_string_lossy();
            let bytes = s.as_bytes();
            writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
            writer.write_all(bytes)?;
        }

        // 3. Entries
        writer.write_all(&(read_guard.entries.len() as u64).to_le_bytes())?;
        for entry in &read_guard.entries {
            writer.write_all(&entry.dir_id.to_le_bytes())?;
            let name_bytes = entry.name.as_bytes();
            writer.write_all(&(name_bytes.len() as u16).to_le_bytes())?;
            writer.write_all(name_bytes)?;
            writer.write_all(&entry.size.to_le_bytes())?;
            writer.write_all(&entry.modified.to_le_bytes())?;
            writer.write_all(&[entry.flags])?;
        }

        writer.flush()?;
        Ok(())
    }

    /// Ultra-fast binary deserialization cache with JSON fallback
    pub fn load_cache(&self, cache_file: &Path) -> io::Result<usize> {
        let file = File::open(cache_file)?;
        let mut reader = BufReader::with_capacity(128 * 1024, file);

        let mut magic = [0u8; 4];
        if reader.read_exact(&mut magic).is_err() {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Empty cache file"));
        }

        if &magic == CACHE_MAGIC {
            // Binary cache format
            let mut ver_bytes = [0u8; 4];
            reader.read_exact(&mut ver_bytes)?;
            let _version = u32::from_le_bytes(ver_bytes);

            // Read directories
            let mut count_bytes = [0u8; 4];
            reader.read_exact(&mut count_bytes)?;
            let dir_count = u32::from_le_bytes(count_bytes) as usize;

            let mut dir_store = DirectoryStore::new();
            for _ in 0..dir_count {
                let mut len_bytes = [0u8; 4];
                reader.read_exact(&mut len_bytes)?;
                let len = u32::from_le_bytes(len_bytes) as usize;
                let mut str_buf = vec![0u8; len];
                reader.read_exact(&mut str_buf)?;
                let dir_str = String::from_utf8_lossy(&str_buf);
                dir_store.get_or_insert(Path::new(dir_str.as_ref()));
            }

            // Read entries
            let mut entries_count_bytes = [0u8; 8];
            reader.read_exact(&mut entries_count_bytes)?;
            let entry_count = u64::from_le_bytes(entries_count_bytes) as usize;

            let mut entries = Vec::with_capacity(entry_count);
            for _ in 0..entry_count {
                let mut dir_id_bytes = [0u8; 4];
                reader.read_exact(&mut dir_id_bytes)?;
                let dir_id = u32::from_le_bytes(dir_id_bytes);

                let mut name_len_bytes = [0u8; 2];
                reader.read_exact(&mut name_len_bytes)?;
                let name_len = u16::from_le_bytes(name_len_bytes) as usize;

                let mut name_buf = vec![0u8; name_len];
                reader.read_exact(&mut name_buf)?;
                let name = String::from_utf8_lossy(&name_buf).into_owned().into_boxed_str();

                let mut size_bytes = [0u8; 8];
                reader.read_exact(&mut size_bytes)?;
                let size = u64::from_le_bytes(size_bytes);

                let mut mod_bytes = [0u8; 4];
                reader.read_exact(&mut mod_bytes)?;
                let modified = u32::from_le_bytes(mod_bytes);

                let mut flag_bytes = [0u8; 1];
                reader.read_exact(&mut flag_bytes)?;
                let flags = flag_bytes[0];

                entries.push(CompactEntry { dir_id, name, size, modified, flags });
            }

            let count = entries.len();
            let mut write_guard = self.data.write();
            write_guard.entries = entries;
            write_guard.dirs = dir_store;
            write_guard.rebuild_entry_index();
            Ok(count)
        } else {
            // Legacy JSON fallback
            let content = fs::read_to_string(cache_file)?;
            let legacy_entries: Vec<FileEntry> =
                serde_json::from_str(&content).map_err(io::Error::other)?;

            let mut dir_store = DirectoryStore::new();
            let mut compact_entries = Vec::with_capacity(legacy_entries.len());

            for fe in legacy_entries {
                let dir_id = dir_store.get_or_insert(&fe.parent);
                let mut flags = 0u8;
                if fe.is_dir {
                    flags |= FLAG_IS_DIR;
                }
                compact_entries.push(CompactEntry {
                    dir_id,
                    name: fe.name.into_boxed_str(),
                    size: fe.size,
                    modified: fe.modified as u32,
                    flags,
                });
            }

            let count = compact_entries.len();
            let mut write_guard = self.data.write();
            write_guard.entries = compact_entries;
            write_guard.dirs = dir_store;
            write_guard.rebuild_entry_index();
            Ok(count)
        }
    }
}
