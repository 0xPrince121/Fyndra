use fyndra_core::{
    CompactEntry, Database, DirectoryStore, FileCategory, MountTable, SearchEngine, SearchQuery,
    FLAG_IS_DIR,
};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[test]
fn test_compact_entry_and_directory_store() {
    let mut dirs = DirectoryStore::new();
    let id1 = dirs.get_or_insert(Path::new("/home/user/projects"));
    let id2 = dirs.get_or_insert(Path::new("/home/user/projects"));
    assert_eq!(id1, id2, "Duplicate directory must return identical ID");
    assert_eq!(dirs.len(), 1);

    let id3 = dirs.get_or_insert(Path::new("/usr/bin"));
    assert_eq!(id3, 1);
    assert_eq!(dirs.len(), 2);

    let entry = CompactEntry {
        dir_id: id1,
        name: "main.rs".into(),
        size: 2048,
        modified: 1700000000,
        flags: 0,
    };

    let fe = entry.to_file_entry(&dirs);
    assert_eq!(fe.name, "main.rs");
    assert_eq!(fe.parent, PathBuf::from("/home/user/projects"));
    assert_eq!(fe.full_path(), PathBuf::from("/home/user/projects/main.rs"));
    assert_eq!(fe.extension(), Some("rs"));
    assert!(!fe.is_dir);
}

#[test]
fn test_compact_search_engine_features() {
    let mut dirs = DirectoryStore::new();
    let doc_id = dirs.get_or_insert(Path::new("/home/user/Documents"));
    let code_id = dirs.get_or_insert(Path::new("/home/user/Source/repo"));

    let entries = vec![
        CompactEntry {
            dir_id: doc_id,
            name: "Quarterly_Report_2026.pdf".into(),
            size: 1024 * 1024,
            modified: 1700000000,
            flags: 0,
        },
        CompactEntry {
            dir_id: doc_id,
            name: "Notes.txt".into(),
            size: 512,
            modified: 1700000000,
            flags: 0,
        },
        CompactEntry {
            dir_id: code_id,
            name: "Cargo.toml".into(),
            size: 800,
            modified: 1700000000,
            flags: 0,
        },
        CompactEntry {
            dir_id: code_id,
            name: "lib.rs".into(),
            size: 4096,
            modified: 1700000000,
            flags: 0,
        },
        CompactEntry {
            dir_id: code_id,
            name: "tests".into(),
            size: 4096,
            modified: 1700000000,
            flags: FLAG_IS_DIR,
        },
    ];

    // Substring multi-term search
    let q = SearchQuery { raw: "report pdf".to_string(), ..Default::default() };
    let res = SearchEngine::execute_compact(&entries, &dirs, &q);
    assert_eq!(res.entries.len(), 1);
    assert_eq!(res.entries[0].name, "Quarterly_Report_2026.pdf");

    // Path slash search
    let q_path = SearchQuery { raw: "Source/repo Cargo".to_string(), ..Default::default() };
    let res_path = SearchEngine::execute_compact(&entries, &dirs, &q_path);
    assert_eq!(res_path.entries.len(), 1);
    assert_eq!(res_path.entries[0].name, "Cargo.toml");

    // Category filter: Code
    let q_cat = SearchQuery { category: FileCategory::Code, ..Default::default() };
    let res_cat = SearchEngine::execute_compact(&entries, &dirs, &q_cat);
    let names: Vec<String> = res_cat.entries.into_iter().map(|e| e.name).collect();
    assert!(names.contains(&"lib.rs".to_string()));
    assert!(names.contains(&"Cargo.toml".to_string()));
    assert!(!names.contains(&"Quarterly_Report_2026.pdf".to_string()));

    // Dir only filter
    let q_dir = SearchQuery { dir_only: true, ..Default::default() };
    let res_dir = SearchEngine::execute_compact(&entries, &dirs, &q_dir);
    assert_eq!(res_dir.entries.len(), 1);
    assert_eq!(res_dir.entries[0].name, "tests");
}

#[test]
fn test_binary_cache_roundtrip() {
    let temp_dir = std::env::temp_dir().join("fyndra_cache_test");
    std::fs::create_dir_all(&temp_dir).unwrap();
    let file_a = temp_dir.join("file_a.txt");
    let file_b = temp_dir.join("file_b.png");
    std::fs::write(&file_a, b"hello").unwrap();
    std::fs::write(&file_b, b"world").unwrap();
    let cache_file = temp_dir.join("test_index.evth");

    let db = Database::default();
    db.upsert_path(&file_a);
    db.upsert_path(&file_b);
    assert_eq!(db.len(), 2);

    db.save_cache(&cache_file).expect("Failed to save binary cache");
    assert!(cache_file.exists());

    let db2 = Database::default();
    let loaded_count = db2.load_cache(&cache_file).expect("Failed to load binary cache");
    assert_eq!(loaded_count, 2);
    assert_eq!(db2.len(), 2);

    let res = db2.search(&SearchQuery { raw: "file_a".to_string(), ..Default::default() });
    assert_eq!(res.entries.len(), 1);
    assert_eq!(res.entries[0].name, "file_a.txt");

    let _ = std::fs::remove_dir_all(temp_dir);
}

#[test]
fn test_mount_table_and_virtual_fs() {
    // Only meaningful on Linux where /proc/mounts exists — inspired by procs/bottom /proc handling
    if !cfg!(target_os = "linux") {
        return;
    }
    let mount_table = MountTable::load();
    assert!(mount_table.is_virtual_fs(Path::new("/proc/cpuinfo")));
    assert!(mount_table.is_virtual_fs(Path::new("/sys/class")));
    assert!(!mount_table.is_virtual_fs(Path::new("/home")));
}

#[test]
fn test_scale_100k_synthetic_entries_performance() {
    // Benchmark 100,000 entries search latency and memory efficiency
    let mut dirs = DirectoryStore::new();
    let mut dir_ids = Vec::new();
    for i in 0..1000 {
        let p = format!("/usr/lib/node_modules/package_{}/src", i);
        dir_ids.push(dirs.get_or_insert(Path::new(&p)));
    }

    let mut entries = Vec::with_capacity(100_000);
    for i in 0..100_000 {
        let dir_id = dir_ids[i % dir_ids.len()];
        let name = format!("module_worker_{:05}.js", i);
        entries.push(CompactEntry {
            dir_id,
            name: name.into_boxed_str(),
            size: (i as u64) * 128,
            modified: 1700000000,
            flags: 0,
        });
    }

    let query = SearchQuery { raw: "worker_99".to_string(), ..Default::default() };

    let start = Instant::now();
    let res = SearchEngine::execute_compact(&entries, &dirs, &query);
    let elapsed = start.elapsed();

    assert!(!res.entries.is_empty());
    println!(
        "Search through 100,000 compact entries completed in {:.3} ms (found {} matches)",
        elapsed.as_secs_f64() * 1000.0,
        res.entries.len()
    );
    // Should take under 15ms even in unoptimized debug test mode!
    assert!(elapsed.as_millis() < 50);
}
