use regex::RegexSet;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Known virtual / pseudo filesystems in Linux that should not be indexed
pub const VIRTUAL_FS_TYPES: &[&str] = &[
    "proc",
    "sysfs",
    "devtmpfs",
    "devpts",
    "cgroup",
    "cgroup2",
    "pstore",
    "bpf",
    "configfs",
    "securityfs",
    "debugfs",
    "tracefs",
    "hugetlbfs",
    "mqueue",
    "fusectl",
    "autofs",
    "ramfs",
    "rpc_pipefs",
    "binfmt_misc",
    "efivarfs",
    "nsfs",
];

/// Storage filesystem types commonly encountered
pub const REAL_FS_TYPES: &[&str] = &[
    "ext4", "ext3", "ext2", "btrfs", "xfs", "f2fs", "zfs", "vfat", "msdos", "fat", "exfat", "ntfs",
    "ntfs3", "fuseblk", "hfsplus", "iso9660", "udf", "reiserfs",
];

/// Known snapshot or internal container paths to skip to avoid indexing millions of duplicates
pub const EXCLUDED_SUBPATH_PATTERNS: &[&str] = &[
    "/.snapshots",
    "/@snapshots",
    "/.btrfs-snapshots",
    "/timeshift/snapshots",
    "/var/lib/docker",
    "/var/lib/containerd",
    "/var/lib/flatpak/runtime",
    "/snap/",
];

/// Compiled RegexSet for EXCLUDED_SUBPATH_PATTERNS — inspired by ripgrep's RegexSet for gitignore.
/// Single DFA pass replaces 7x `contains` per file (50M files = 350M scans → ~50M).
static EXCLUDED_RE: OnceLock<RegexSet> = OnceLock::new();

fn excluded_set() -> &'static RegexSet {
    EXCLUDED_RE.get_or_init(|| {
        // Escape each pattern as literal substring search
        let escaped: Vec<String> =
            EXCLUDED_SUBPATH_PATTERNS.iter().map(|p| regex::escape(p)).collect();
        RegexSet::new(&escaped).expect("valid excluded patterns")
    })
}

/// Fast single-pass excluded subpath check — replaces linear `contains` loop in crawler.
#[inline]
pub fn is_excluded_subpath(path: &str) -> bool {
    excluded_set().is_match(path)
}

#[derive(Debug, Clone)]
pub struct MountEntry {
    pub device: String,
    pub mount_point: PathBuf,
    pub fs_type: String,
    pub options: HashSet<String>,
    pub is_removable: bool,
    pub is_btrfs: bool,
    pub is_real_storage: bool,
}

#[derive(Debug, Clone, Default)]
pub struct MountTable {
    pub entries: Vec<MountEntry>,
}

impl MountTable {
    /// Loads current mount points from /proc/mounts (or fallback).
    /// Linux-only: on non-Linux returns empty (virtual FS checks then harmlessly false) — cfg guard mirrors bottom/procs per-OS modules.
    pub fn load() -> Self {
        if !cfg!(target_os = "linux") {
            return Self { entries: Vec::new() };
        }
        let mut entries = Vec::new();
        let file = match File::open("/proc/mounts") {
            Ok(f) => f,
            Err(_) => return Self { entries },
        };

        let reader = BufReader::new(file);
        for line in reader.lines().map_while(Result::ok) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 4 {
                continue;
            }

            let device = parts[0].to_string();
            let mount_point = PathBuf::from(parts[1]);
            let fs_type = parts[2].to_string();
            let options: HashSet<String> = parts[3].split(',').map(|s| s.to_string()).collect();

            let is_btrfs = fs_type == "btrfs";
            let is_real_storage = REAL_FS_TYPES.contains(&fs_type.as_str());

            // Check if mounted under typical removable drive paths
            let is_removable = mount_point.starts_with("/media")
                || mount_point.starts_with("/run/media")
                || mount_point.starts_with("/mnt")
                || options.contains("user")
                || options.contains("users");

            entries.push(MountEntry {
                device,
                mount_point,
                fs_type,
                options,
                is_removable,
                is_btrfs,
                is_real_storage,
            });
        }

        // Sort by mount_point length descending so is_virtual_fs finds longest prefix first and early-exits.
        // Inspired by ripgrep's longest-prefix match optimization for gitignore.
        entries.sort_by_key(|a| std::cmp::Reverse(a.mount_point.as_os_str().len()));

        Self { entries }
    }

    /// Checks if a path is located on a virtual/pseudo filesystem
    /// Optimized: entries are pre-sorted longest-first, so first prefix match is best.
    pub fn is_virtual_fs(&self, path: &Path) -> bool {
        for entry in &self.entries {
            if path.starts_with(&entry.mount_point) {
                return VIRTUAL_FS_TYPES.contains(&entry.fs_type.as_str());
            }
        }
        false
    }

    /// Checks if a path is on a removable storage device
    pub fn is_removable_drive(&self, path: &Path) -> bool {
        path.starts_with("/media") || path.starts_with("/run/media") || path.starts_with("/mnt")
    }

    /// Returns list of all active removable storage mount points
    pub fn removable_mounts(&self) -> Vec<PathBuf> {
        self.entries
            .iter()
            .filter(|e| e.is_removable && e.is_real_storage)
            .map(|e| e.mount_point.clone())
            .collect()
    }
}
