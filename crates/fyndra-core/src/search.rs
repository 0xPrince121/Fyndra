use crate::entry::{CompactEntry, DirectoryStore, FileEntry};
use glob::Pattern;
use rayon::prelude::*;
use regex::Regex;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileCategory {
    #[default]
    All,
    Folders,
    Documents,
    Images,
    Audio,
    Video,
    Archives,
    Code,
}

impl FileCategory {
    #[inline]
    pub fn matches_compact(&self, entry: &CompactEntry) -> bool {
        match self {
            FileCategory::All => true,
            FileCategory::Folders => entry.is_dir(),
            FileCategory::Documents => {
                if entry.is_dir() {
                    return false;
                }
                let ext = entry.extension().unwrap_or("");
                is_document_ext(ext)
            }
            FileCategory::Images => {
                if entry.is_dir() {
                    return false;
                }
                let ext = entry.extension().unwrap_or("");
                is_image_ext(ext)
            }
            FileCategory::Audio => {
                if entry.is_dir() {
                    return false;
                }
                let ext = entry.extension().unwrap_or("");
                is_audio_ext(ext)
            }
            FileCategory::Video => {
                if entry.is_dir() {
                    return false;
                }
                let ext = entry.extension().unwrap_or("");
                is_video_ext(ext)
            }
            FileCategory::Archives => {
                if entry.is_dir() {
                    return false;
                }
                let ext = entry.extension().unwrap_or("");
                is_archive_ext(ext)
            }
            FileCategory::Code => {
                if entry.is_dir() {
                    return false;
                }
                let ext = entry.extension().unwrap_or("");
                is_code_ext(ext)
            }
        }
    }

    #[inline]
    pub fn matches(&self, entry: &FileEntry) -> bool {
        match self {
            FileCategory::All => true,
            FileCategory::Folders => entry.is_dir,
            FileCategory::Documents => {
                if entry.is_dir {
                    return false;
                }
                let ext = entry.extension().unwrap_or("");
                is_document_ext(ext)
            }
            FileCategory::Images => {
                if entry.is_dir {
                    return false;
                }
                let ext = entry.extension().unwrap_or("");
                is_image_ext(ext)
            }
            FileCategory::Audio => {
                if entry.is_dir {
                    return false;
                }
                let ext = entry.extension().unwrap_or("");
                is_audio_ext(ext)
            }
            FileCategory::Video => {
                if entry.is_dir {
                    return false;
                }
                let ext = entry.extension().unwrap_or("");
                is_video_ext(ext)
            }
            FileCategory::Archives => {
                if entry.is_dir {
                    return false;
                }
                let ext = entry.extension().unwrap_or("");
                is_archive_ext(ext)
            }
            FileCategory::Code => {
                if entry.is_dir {
                    return false;
                }
                let ext = entry.extension().unwrap_or("");
                is_code_ext(ext)
            }
        }
    }
}

#[inline]
fn is_document_ext(ext: &str) -> bool {
    let mut buf = [0u8; 16];
    let ext_lower = to_lower_ascii_slice(ext, &mut buf);
    matches!(
        ext_lower,
        "pdf"
            | "doc"
            | "docx"
            | "txt"
            | "md"
            | "odt"
            | "rtf"
            | "epub"
            | "csv"
            | "xlsx"
            | "pptx"
            | "ods"
            | "log"
    )
}

#[inline]
fn is_image_ext(ext: &str) -> bool {
    let mut buf = [0u8; 16];
    let ext_lower = to_lower_ascii_slice(ext, &mut buf);
    matches!(
        ext_lower,
        "png" | "jpg" | "jpeg" | "svg" | "gif" | "webp" | "bmp" | "ico" | "tiff" | "raw" | "heic"
    )
}

#[inline]
fn is_audio_ext(ext: &str) -> bool {
    let mut buf = [0u8; 16];
    let ext_lower = to_lower_ascii_slice(ext, &mut buf);
    matches!(ext_lower, "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" | "wma" | "opus")
}

#[inline]
fn is_video_ext(ext: &str) -> bool {
    let mut buf = [0u8; 16];
    let ext_lower = to_lower_ascii_slice(ext, &mut buf);
    matches!(ext_lower, "mp4" | "mkv" | "webm" | "avi" | "mov" | "flv" | "wmv" | "m4v")
}

#[inline]
fn is_archive_ext(ext: &str) -> bool {
    let mut buf = [0u8; 16];
    let ext_lower = to_lower_ascii_slice(ext, &mut buf);
    matches!(
        ext_lower,
        "zip" | "tar" | "gz" | "7z" | "rar" | "xz" | "bz2" | "iso" | "deb" | "rpm" | "zst"
    )
}

#[inline]
fn is_code_ext(ext: &str) -> bool {
    let mut buf = [0u8; 16];
    let ext_lower = to_lower_ascii_slice(ext, &mut buf);
    matches!(
        ext_lower,
        "rs" | "py"
            | "c"
            | "cpp"
            | "h"
            | "hpp"
            | "js"
            | "ts"
            | "jsx"
            | "tsx"
            | "html"
            | "css"
            | "json"
            | "toml"
            | "yaml"
            | "yml"
            | "sh"
            | "bash"
            | "go"
            | "java"
            | "kt"
            | "swift"
            | "sql"
            | "xml"
            | "php"
    )
}

/// Zero-allocation ASCII lowercase helper for stack buffer
#[inline]
fn to_lower_ascii_slice<'a>(s: &'a str, buf: &'a mut [u8; 16]) -> &'a str {
    let bytes = s.as_bytes();
    let len = bytes.len().min(buf.len());
    for i in 0..len {
        buf[i] = bytes[i].to_ascii_lowercase();
    }
    std::str::from_utf8(&buf[..len]).unwrap_or("")
}

/// Fast case-insensitive substring search without heap allocations
#[inline]
pub fn contains_ignore_case(haystack: &str, needle_lower: &str) -> bool {
    if needle_lower.is_empty() {
        return true;
    }
    if haystack.is_ascii() && needle_lower.is_ascii() {
        let h_bytes = haystack.as_bytes();
        let n_bytes = needle_lower.as_bytes();
        if n_bytes.len() > h_bytes.len() {
            return false;
        }
        h_bytes.windows(n_bytes.len()).any(|window| {
            window.iter().zip(n_bytes.iter()).all(|(&h, &n)| h.to_ascii_lowercase() == n)
        })
    } else {
        haystack.to_lowercase().contains(needle_lower)
    }
}

#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub raw: String,
    pub case_sensitive: bool,
    pub is_regex: bool,
    pub is_wildcard: bool,
    pub dir_only: bool,
    pub file_only: bool,
    pub category: FileCategory,
    pub max_results: usize,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            raw: String::new(),
            case_sensitive: false,
            is_regex: false,
            is_wildcard: false,
            dir_only: false,
            file_only: false,
            category: FileCategory::All,
            max_results: 10_000,
        }
    }
}

pub struct SearchResult {
    pub entries: Vec<FileEntry>,
    pub elapsed_micros: u128,
    pub total_scanned: usize,
}

pub struct SearchEngine;

impl SearchEngine {
    /// Ultra-fast parallel search over memory-efficient CompactEntry dataset
    pub fn execute_compact(
        entries: &[CompactEntry],
        dirs: &DirectoryStore,
        query: &SearchQuery,
    ) -> SearchResult {
        let start = Instant::now();

        // 1. Empty query (or pure filter mode)
        if query.raw.is_empty() {
            if query.category != FileCategory::All || query.dir_only || query.file_only {
                let matched: Vec<FileEntry> = entries
                    .par_iter()
                    .filter(|e| {
                        if query.dir_only && !e.is_dir() {
                            return false;
                        }
                        if query.file_only && e.is_dir() {
                            return false;
                        }
                        query.category.matches_compact(e)
                    })
                    .take_any(query.max_results)
                    .map(|c| c.to_file_entry(dirs))
                    .collect();

                return SearchResult {
                    entries: matched,
                    elapsed_micros: start.elapsed().as_micros(),
                    total_scanned: entries.len(),
                };
            }

            let slice = if entries.len() > query.max_results {
                &entries[..query.max_results]
            } else {
                entries
            };
            let matched: Vec<FileEntry> = slice.iter().map(|c| c.to_file_entry(dirs)).collect();

            return SearchResult {
                entries: matched,
                elapsed_micros: start.elapsed().as_micros(),
                total_scanned: entries.len(),
            };
        }

        let has_slash = query.raw.contains('/');

        // 2. Regex Mode
        if query.is_regex {
            let regex_result = if query.case_sensitive {
                Regex::new(&query.raw)
            } else {
                Regex::new(&format!("(?i){}", query.raw))
            };

            if let Ok(re) = regex_result {
                let matched: Vec<FileEntry> = entries
                    .par_iter()
                    .filter(|e| {
                        if query.dir_only && !e.is_dir() {
                            return false;
                        }
                        if query.file_only && e.is_dir() {
                            return false;
                        }
                        if !query.category.matches_compact(e) {
                            return false;
                        }

                        if has_slash {
                            let parent = dirs
                                .get_path_ref(e.dir_id)
                                .unwrap_or_else(|| std::path::Path::new("/"));
                            let full = parent.join(&*e.name);
                            re.is_match(&full.to_string_lossy())
                        } else {
                            re.is_match(&e.name)
                        }
                    })
                    .take_any(query.max_results)
                    .map(|c| c.to_file_entry(dirs))
                    .collect();

                return SearchResult {
                    entries: matched,
                    elapsed_micros: start.elapsed().as_micros(),
                    total_scanned: entries.len(),
                };
            }
        }

        // 3. Wildcard Mode (* or ?)
        let contains_wildcards = query.raw.contains('*') || query.raw.contains('?');
        if query.is_wildcard || contains_wildcards {
            let pattern_str =
                if query.case_sensitive { query.raw.clone() } else { query.raw.to_lowercase() };

            if let Ok(glob_pattern) = Pattern::new(&pattern_str) {
                let matched: Vec<FileEntry> = entries
                    .par_iter()
                    .filter(|e| {
                        if query.dir_only && !e.is_dir() {
                            return false;
                        }
                        if query.file_only && e.is_dir() {
                            return false;
                        }
                        if !query.category.matches_compact(e) {
                            return false;
                        }

                        if has_slash {
                            let parent = dirs
                                .get_path_ref(e.dir_id)
                                .unwrap_or_else(|| std::path::Path::new("/"));
                            let full = parent.join(&*e.name);
                            let target = if query.case_sensitive {
                                full.to_string_lossy().to_string()
                            } else {
                                full.to_string_lossy().to_lowercase()
                            };
                            glob_pattern.matches(&target)
                        } else if query.case_sensitive {
                            glob_pattern.matches(&e.name)
                        } else {
                            glob_pattern.matches(&e.name.to_lowercase())
                        }
                    })
                    .take_any(query.max_results)
                    .map(|c| c.to_file_entry(dirs))
                    .collect();

                return SearchResult {
                    entries: matched,
                    elapsed_micros: start.elapsed().as_micros(),
                    total_scanned: entries.len(),
                };
            }
        }

        // 4. Ultra-Fast Substring Matching (Default Mode)
        let query_term =
            if query.case_sensitive { query.raw.clone() } else { query.raw.to_lowercase() };

        let terms: Vec<&str> = query_term.split_whitespace().collect();

        let matched: Vec<FileEntry> = entries
            .par_iter()
            .filter(|e| {
                if query.dir_only && !e.is_dir() {
                    return false;
                }
                if query.file_only && e.is_dir() {
                    return false;
                }
                if !query.category.matches_compact(e) {
                    return false;
                }

                if terms.is_empty() {
                    return true;
                }

                if has_slash {
                    let parent =
                        dirs.get_path_ref(e.dir_id).unwrap_or_else(|| std::path::Path::new("/"));
                    let full = parent.join(&*e.name);
                    let full_lossy = full.to_string_lossy();
                    if query.case_sensitive {
                        terms.iter().all(|&t| full_lossy.contains(t))
                    } else {
                        terms.iter().all(|&t| contains_ignore_case(&full_lossy, t))
                    }
                } else if query.case_sensitive {
                    terms.iter().all(|&t| e.name.contains(t))
                } else {
                    terms.iter().all(|&t| contains_ignore_case(&e.name, t))
                }
            })
            .take_any(query.max_results)
            .map(|c| c.to_file_entry(dirs))
            .collect();

        SearchResult {
            entries: matched,
            elapsed_micros: start.elapsed().as_micros(),
            total_scanned: entries.len(),
        }
    }

    /// Legacy search execution over FileEntry slice
    pub fn execute(entries: &[FileEntry], query: &SearchQuery) -> SearchResult {
        let start = Instant::now();

        if query.raw.is_empty() {
            if query.category != FileCategory::All || query.dir_only || query.file_only {
                let matched: Vec<FileEntry> = entries
                    .par_iter()
                    .filter(|e| {
                        if query.dir_only && !e.is_dir {
                            return false;
                        }
                        if query.file_only && e.is_dir {
                            return false;
                        }
                        query.category.matches(e)
                    })
                    .take_any(query.max_results)
                    .cloned()
                    .collect();

                return SearchResult {
                    entries: matched,
                    elapsed_micros: start.elapsed().as_micros(),
                    total_scanned: entries.len(),
                };
            }

            let slice = if entries.len() > query.max_results {
                &entries[..query.max_results]
            } else {
                entries
            };
            return SearchResult {
                entries: slice.to_vec(),
                elapsed_micros: start.elapsed().as_micros(),
                total_scanned: entries.len(),
            };
        }

        let has_slash = query.raw.contains('/');

        if query.is_regex {
            let regex_result = if query.case_sensitive {
                Regex::new(&query.raw)
            } else {
                Regex::new(&format!("(?i){}", query.raw))
            };

            if let Ok(re) = regex_result {
                let matched: Vec<FileEntry> = entries
                    .par_iter()
                    .filter(|e| {
                        if query.dir_only && !e.is_dir {
                            return false;
                        }
                        if query.file_only && e.is_dir {
                            return false;
                        }
                        if !query.category.matches(e) {
                            return false;
                        }
                        if has_slash {
                            re.is_match(&e.full_path().to_string_lossy())
                        } else {
                            re.is_match(&e.name)
                        }
                    })
                    .take_any(query.max_results)
                    .cloned()
                    .collect();

                return SearchResult {
                    entries: matched,
                    elapsed_micros: start.elapsed().as_micros(),
                    total_scanned: entries.len(),
                };
            }
        }

        let contains_wildcards = query.raw.contains('*') || query.raw.contains('?');
        if query.is_wildcard || contains_wildcards {
            let pattern_str =
                if query.case_sensitive { query.raw.clone() } else { query.raw.to_lowercase() };

            if let Ok(glob_pattern) = Pattern::new(&pattern_str) {
                let matched: Vec<FileEntry> = entries
                    .par_iter()
                    .filter(|e| {
                        if query.dir_only && !e.is_dir {
                            return false;
                        }
                        if query.file_only && e.is_dir {
                            return false;
                        }
                        if !query.category.matches(e) {
                            return false;
                        }

                        if has_slash {
                            let target = if query.case_sensitive {
                                e.full_path().to_string_lossy().to_string()
                            } else {
                                e.full_path().to_string_lossy().to_lowercase()
                            };
                            glob_pattern.matches(&target)
                        } else {
                            let target = if query.case_sensitive { &e.name } else { &e.name_lower };
                            glob_pattern.matches(target)
                        }
                    })
                    .take_any(query.max_results)
                    .cloned()
                    .collect();

                return SearchResult {
                    entries: matched,
                    elapsed_micros: start.elapsed().as_micros(),
                    total_scanned: entries.len(),
                };
            }
        }

        let query_term =
            if query.case_sensitive { query.raw.clone() } else { query.raw.to_lowercase() };

        let terms: Vec<&str> = query_term.split_whitespace().collect();

        let matched: Vec<FileEntry> = entries
            .par_iter()
            .filter(|e| {
                if query.dir_only && !e.is_dir {
                    return false;
                }
                if query.file_only && e.is_dir {
                    return false;
                }
                if !query.category.matches(e) {
                    return false;
                }

                if terms.is_empty() {
                    return true;
                }

                if has_slash {
                    let full_lossy = e.full_path().to_string_lossy().to_string();
                    if query.case_sensitive {
                        terms.iter().all(|&t| full_lossy.contains(t))
                    } else {
                        let full_lower = full_lossy.to_lowercase();
                        terms.iter().all(|&t| full_lower.contains(t))
                    }
                } else if query.case_sensitive {
                    terms.iter().all(|&t| e.name.contains(t))
                } else {
                    terms.iter().all(|&t| e.name_lower.contains(t))
                }
            })
            .take_any(query.max_results)
            .cloned()
            .collect();

        SearchResult {
            entries: matched,
            elapsed_micros: start.elapsed().as_micros(),
            total_scanned: entries.len(),
        }
    }
}
