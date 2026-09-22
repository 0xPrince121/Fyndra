pub mod crawler;
pub mod entry;
pub mod index;
pub mod mounts;
pub mod paths;
pub mod search;
pub mod watcher;

pub use crawler::{CrawlStats, Crawler, CrawlerConfig, DEFAULT_EXCLUDED_ROOTS};
pub use entry::{
    CompactEntry, DirectoryStore, FileEntry, FLAG_IS_DIR, FLAG_IS_REMOVABLE, FLAG_IS_SYMLINK,
};
pub use index::{Database, IndexData};
pub use mounts::{MountEntry, MountTable, REAL_FS_TYPES, VIRTUAL_FS_TYPES};
pub use paths::{
    cache_dir, cache_file, config_dir, config_file, data_dir, ensure_dirs, migrate_legacy,
};
pub use search::{FileCategory, SearchEngine, SearchQuery, SearchResult};
pub use watcher::{FileWatcher, WatcherHealth};
