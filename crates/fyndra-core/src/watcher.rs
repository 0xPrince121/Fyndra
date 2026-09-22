use crate::index::Database;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatcherHealth {
    Healthy,
    PartiallyDegraded(usize), // ENOSPC or permission limit hit
    Failed,
}

pub struct FileWatcher {
    _watcher_thread: Option<thread::JoinHandle<()>>,
    is_running: Arc<AtomicBool>,
    pub health: WatcherHealth,
}

impl FileWatcher {
    /// Starts a background watcher thread that updates the database on filesystem changes.
    /// Gracefully handles Linux inotify max_user_watches limits (ENOSPC) and permissions.
    pub fn start(db: Database, watch_roots: Vec<PathBuf>) -> notify::Result<Self> {
        let (tx, rx) = channel::<notify::Result<Event>>();
        let is_running = Arc::new(AtomicBool::new(true));

        let mut watcher = notify::recommended_watcher(tx)?;
        let mut watched_count = 0;
        let mut hit_limit = false;

        for root in &watch_roots {
            if !root.exists() {
                continue;
            }

            match watcher.watch(root, RecursiveMode::Recursive) {
                Ok(_) => {
                    watched_count += 1;
                }
                Err(e) => {
                    // Check if error was ENOSPC (No space left on device - inotify watches full)
                    eprintln!(
                        "Warning: FileWatcher could not recursively watch {:?}: {} (falling back to parent)",
                        root, e
                    );
                    hit_limit = true;

                    // Fallback to NonRecursive watch on the top directory so top-level changes are still caught
                    let _ = watcher.watch(root, RecursiveMode::NonRecursive);
                }
            }
        }

        let health = if hit_limit {
            WatcherHealth::PartiallyDegraded(watched_count)
        } else if watched_count == 0 && !watch_roots.is_empty() {
            WatcherHealth::Failed
        } else {
            WatcherHealth::Healthy
        };

        let running_flag = Arc::clone(&is_running);
        let handle = thread::spawn(move || {
            let _watcher = watcher;
            let mut upsert_buffer = Vec::with_capacity(512);
            let mut remove_buffer = Vec::with_capacity(512);

            while running_flag.load(Ordering::Relaxed) {
                match rx.recv_timeout(Duration::from_millis(150)) {
                    Ok(Ok(event)) => {
                        match event.kind {
                            EventKind::Create(_) | EventKind::Modify(_) => {
                                upsert_buffer.extend(event.paths);
                            }
                            EventKind::Remove(_) => {
                                remove_buffer.extend(event.paths);
                            }
                            _ => {}
                        }

                        // Flush when batch accumulates
                        if upsert_buffer.len() >= 500 {
                            db.batch_upsert(&upsert_buffer);
                            upsert_buffer.clear();
                        }
                        if remove_buffer.len() >= 500 {
                            db.batch_remove(&remove_buffer);
                            remove_buffer.clear();
                        }
                    }
                    Ok(Err(_)) => {}
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        // Flush during quiescence
                        if !upsert_buffer.is_empty() {
                            db.batch_upsert(&upsert_buffer);
                            upsert_buffer.clear();
                        }
                        if !remove_buffer.is_empty() {
                            db.batch_remove(&remove_buffer);
                            remove_buffer.clear();
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        break;
                    }
                }
            }

            // Final flush
            if !upsert_buffer.is_empty() {
                db.batch_upsert(&upsert_buffer);
            }
            if !remove_buffer.is_empty() {
                db.batch_remove(&remove_buffer);
            }
        });

        Ok(Self { _watcher_thread: Some(handle), is_running, health })
    }

    pub fn stop(&self) {
        self.is_running.store(false, Ordering::Relaxed);
    }
}
