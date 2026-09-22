use crate::entry::{
    CompactEntry, DirectoryStore, FileEntry, FLAG_IS_DIR, FLAG_IS_REMOVABLE, FLAG_IS_SYMLINK,
};
use crate::mounts::{is_excluded_subpath, MountTable};
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Directories that should never be indexed on Linux
pub const DEFAULT_EXCLUDED_ROOTS: &[&str] = &[
    "/proc",
    "/sys",
    "/dev",
    "/run",
    "/tmp",
    "/var/run",
    "/var/lock",
    "/var/lib/docker",
    "/var/lib/containerd",
    "/.snapshots",
];

pub struct CrawlerConfig {
    pub excluded_paths: HashSet<PathBuf>,
    pub follow_symlinks: bool,
    pub max_depth: Option<usize>,
    pub index_removable_drives: bool,
}

impl Default for CrawlerConfig {
    fn default() -> Self {
        let mut excluded_paths = HashSet::new();
        for &path in DEFAULT_EXCLUDED_ROOTS {
            excluded_paths.insert(PathBuf::from(path));
        }
        Self {
            excluded_paths,
            follow_symlinks: false,
            max_depth: None,
            index_removable_drives: true,
        }
    }
}

pub struct CrawlStats {
    pub total_files: usize,
    pub total_dirs: usize,
    pub elapsed_ms: u128,
}

pub struct Crawler {
    config: CrawlerConfig,
    mount_table: MountTable,
}

/// Internal representation of a directory and its immediate children
struct DirCrawlResult {
    dir: PathBuf,
    entries: Vec<(Box<str>, u64, u32, u8)>, // (name, size, modified, flags)
}

impl Crawler {
    pub fn new(config: CrawlerConfig) -> Self {
        let mount_table = MountTable::load();
        Self { config, mount_table }
    }

    /// Crawls a list of target paths in parallel, returning memory-efficient CompactEntries & DirectoryStore
    pub fn crawl_roots_compact(
        &self,
        roots: &[PathBuf],
    ) -> (Vec<CompactEntry>, DirectoryStore, CrawlStats) {
        let start = Instant::now();
        let total_files = Arc::new(AtomicUsize::new(0));
        let total_dirs = Arc::new(AtomicUsize::new(0));

        let mut active_roots: Vec<PathBuf> = roots.to_vec();

        // Optionally discover and append removable drive mounts (/media, /run/media, /mnt)
        if self.config.index_removable_drives {
            for rem in self.mount_table.removable_mounts() {
                if !active_roots.iter().any(|r| rem.starts_with(r) || r.starts_with(&rem))
                    && rem.exists()
                {
                    active_roots.push(rem);
                }
            }
        }

        // Streaming with bounded 1024 channel + concurrent consumer (C2) — true peak cut: producer Rayon scope streams DirCrawlResult while consumer builds DirectoryStore incrementally, bounded prevents unbounded buffer at 50M
        let (sender, receiver) = crossbeam_channel::bounded::<DirCrawlResult>(1024);
        let dir_store = Arc::new(parking_lot::Mutex::new(DirectoryStore::new()));
        let compact_entries = Arc::new(parking_lot::Mutex::new(Vec::<CompactEntry>::new()));
        let dir_store_consumer = Arc::clone(&dir_store);
        let compact_entries_consumer = Arc::clone(&compact_entries);
        // Concurrent consumer thread drains while producers send — avoids deadlock on bounded full
        let consumer = std::thread::spawn(move || {
            for result in receiver {
                let dir_id = {
                    let mut ds = dir_store_consumer.lock();
                    ds.get_or_insert(&result.dir)
                };
                let mut ce = compact_entries_consumer.lock();
                for (name, size, modified, flags) in result.entries {
                    ce.push(CompactEntry { dir_id, name, size, modified, flags });
                }
            }
        });

        rayon::scope(|s| {
            for root in &active_roots {
                if !root.exists() || self.is_excluded(root) {
                    continue;
                }
                let sender = sender.clone();
                let total_files = Arc::clone(&total_files);
                let total_dirs = Arc::clone(&total_dirs);
                let root = root.clone();
                let this = &self;
                s.spawn(move |_| {
                    let mut ancestor_inodes = HashSet::new();
                    if let Ok(meta) = fs::symlink_metadata(&root) {
                        ancestor_inodes.insert((meta.dev(), meta.ino()));
                    }
                    let results = this.crawl_dir_recursive(
                        &root,
                        0,
                        &mut ancestor_inodes,
                        &total_files,
                        &total_dirs,
                    );
                    for r in results {
                        let _ = sender.send(r);
                    }
                });
            }
        });
        drop(sender);
        consumer.join().unwrap();

        // Extract after consumer done
        let dir_store = Arc::try_unwrap(dir_store).unwrap().into_inner();
        let compact_entries = Arc::try_unwrap(compact_entries).unwrap().into_inner();

        let stats = CrawlStats {
            total_files: total_files.load(Ordering::Relaxed),
            total_dirs: total_dirs.load(Ordering::Relaxed),
            elapsed_ms: start.elapsed().as_millis(),
        };

        (compact_entries, dir_store, stats)
    }

    /// Legacy crawl API returning rich FileEntry structs
    pub fn crawl_roots(&self, roots: &[PathBuf]) -> (Vec<FileEntry>, CrawlStats) {
        let (compact, dir_store, stats) = self.crawl_roots_compact(roots);
        let entries: Vec<FileEntry> =
            compact.into_par_iter().map(|c| c.to_file_entry(&dir_store)).collect();
        (entries, stats)
    }

    fn is_excluded(&self, path: &Path) -> bool {
        if self.config.excluded_paths.contains(path) {
            return true;
        }

        let path_str = path.to_string_lossy();

        // 1. Skip known subpath snapshot / loop patterns — RegexSet single DFA pass (ripgrep)
        if is_excluded_subpath(&path_str) {
            return true;
        }

        // 2. Skip virtual/pseudo filesystems (proc, sysfs, devtmpfs, cgroups)
        if self.mount_table.is_virtual_fs(path) {
            return true;
        }

        false
    }

    fn crawl_dir_recursive(
        &self,
        dir: &Path,
        current_depth: usize,
        ancestor_inodes: &mut HashSet<(u64, u64)>,
        file_counter: &Arc<AtomicUsize>,
        dir_counter: &Arc<AtomicUsize>,
    ) -> Vec<DirCrawlResult> {
        // Safety ceiling against extremely deep trees
        if current_depth > 128 {
            return Vec::new();
        }

        if let Some(max_depth) = self.config.max_depth {
            if current_depth > max_depth {
                return Vec::new();
            }
        }

        // Permission boundary safety: gracefully handle EACCES / EPERM without panics
        let read_dir = match fs::read_dir(dir) {
            Ok(rd) => rd,
            Err(_) => return Vec::new(),
        };

        let is_removable = self.mount_table.is_removable_drive(dir);
        let mut entries = Vec::with_capacity(64);
        let mut subdirs = Vec::new();

        for entry_res in read_dir {
            let entry = match entry_res {
                Ok(e) => e,
                Err(_) => continue, // Permission denied or broken dirent
            };

            let path = entry.path();
            if self.is_excluded(&path) {
                continue;
            }

            // Exploit Linux d_type: zero-syscall file type check!
            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };

            let is_symlink = file_type.is_symlink();
            let is_dir = file_type.is_dir();

            if is_symlink && !self.config.follow_symlinks {
                // When follow_symlinks is disabled, record symlink as a leaf entry without traversing
                let name = entry.file_name().to_string_lossy().into_owned().into_boxed_str();
                let mut flags = FLAG_IS_SYMLINK;
                if is_removable {
                    flags |= FLAG_IS_REMOVABLE;
                }
                entries.push((name, 0, 0, flags));
                file_counter.fetch_add(1, Ordering::Relaxed);
                continue;
            }

            // Extract metadata (size & modified time)
            let (size, modified) = if let Ok(meta) = entry.metadata() {
                let size = if is_dir { 0 } else { meta.len() };
                let modified = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as u32)
                    .unwrap_or(0);
                (size, modified)
            } else {
                (0, 0)
            };

            let name = entry.file_name().to_string_lossy().into_owned().into_boxed_str();
            let mut flags = 0u8;
            if is_dir {
                flags |= FLAG_IS_DIR;
                dir_counter.fetch_add(1, Ordering::Relaxed);
                subdirs.push(path);
            } else {
                file_counter.fetch_add(1, Ordering::Relaxed);
            }
            if is_symlink {
                flags |= FLAG_IS_SYMLINK;
            }
            if is_removable {
                flags |= FLAG_IS_REMOVABLE;
            }

            entries.push((name, size, modified, flags));
        }

        let mut results = Vec::new();
        results.push(DirCrawlResult { dir: dir.to_path_buf(), entries });

        // Parallelize subdirectories using Rayon across CPU cores
        if current_depth < 6 && subdirs.len() > 1 {
            let subdir_results: Vec<DirCrawlResult> = subdirs
                .into_par_iter()
                .flat_map(|sub| {
                    let mut local_ancestors = ancestor_inodes.clone();
                    // Symlink cycle detection: track (dev, ino)
                    if let Ok(meta) = fs::symlink_metadata(&sub) {
                        let id = (meta.dev(), meta.ino());
                        if !local_ancestors.insert(id) {
                            return Vec::new(); // Cycle detected!
                        }
                    }
                    self.crawl_dir_recursive(
                        &sub,
                        current_depth + 1,
                        &mut local_ancestors,
                        file_counter,
                        dir_counter,
                    )
                })
                .collect();
            results.extend(subdir_results);
        } else {
            for sub in subdirs {
                if let Ok(meta) = fs::symlink_metadata(&sub) {
                    let id = (meta.dev(), meta.ino());
                    if !ancestor_inodes.insert(id) {
                        continue; // Cycle detected!
                    }
                    let sub_res = self.crawl_dir_recursive(
                        &sub,
                        current_depth + 1,
                        ancestor_inodes,
                        file_counter,
                        dir_counter,
                    );
                    ancestor_inodes.remove(&id);
                    results.extend(sub_res);
                } else {
                    let sub_res = self.crawl_dir_recursive(
                        &sub,
                        current_depth + 1,
                        ancestor_inodes,
                        file_counter,
                        dir_counter,
                    );
                    results.extend(sub_res);
                }
            }
        }

        results
    }
}
