use fyndra_core::{FileEntry, SearchEngine, SearchQuery};
use std::path::PathBuf;

#[test]
fn test_substring_search() {
    let entries = vec![
        FileEntry {
            name: "report_2026.pdf".to_string(),
            name_lower: "report_2026.pdf".to_string(),
            parent: PathBuf::from("/home/user/documents"),
            size: 1024,
            is_dir: false,
            modified: 1700000000,
        },
        FileEntry {
            name: "photo.jpg".to_string(),
            name_lower: "photo.jpg".to_string(),
            parent: PathBuf::from("/home/user/pictures"),
            size: 2048,
            is_dir: false,
            modified: 1700000000,
        },
        FileEntry {
            name: "documents".to_string(),
            name_lower: "documents".to_string(),
            parent: PathBuf::from("/home/user"),
            size: 4096,
            is_dir: true,
            modified: 1700000000,
        },
    ];

    let q = SearchQuery { raw: "report".to_string(), ..Default::default() };
    let res = SearchEngine::execute(&entries, &q);
    assert_eq!(res.entries.len(), 1);
    assert_eq!(res.entries[0].name, "report_2026.pdf");
}

#[test]
fn test_wildcard_search() {
    let entries = vec![
        FileEntry {
            name: "invoice_jan.pdf".to_string(),
            name_lower: "invoice_jan.pdf".to_string(),
            parent: PathBuf::from("/home/user"),
            size: 512,
            is_dir: false,
            modified: 1700000000,
        },
        FileEntry {
            name: "invoice_feb.docx".to_string(),
            name_lower: "invoice_feb.docx".to_string(),
            parent: PathBuf::from("/home/user"),
            size: 512,
            is_dir: false,
            modified: 1700000000,
        },
    ];

    let q = SearchQuery { raw: "*.pdf".to_string(), is_wildcard: true, ..Default::default() };
    let res = SearchEngine::execute(&entries, &q);
    assert_eq!(res.entries.len(), 1);
    assert_eq!(res.entries[0].name, "invoice_jan.pdf");
}

#[test]
fn test_dir_only_filter() {
    let entries = vec![
        FileEntry {
            name: "test_file.txt".to_string(),
            name_lower: "test_file.txt".to_string(),
            parent: PathBuf::from("/home/user"),
            size: 10,
            is_dir: false,
            modified: 1700000000,
        },
        FileEntry {
            name: "test_folder".to_string(),
            name_lower: "test_folder".to_string(),
            parent: PathBuf::from("/home/user"),
            size: 4096,
            is_dir: true,
            modified: 1700000000,
        },
    ];

    let q = SearchQuery { raw: "test".to_string(), dir_only: true, ..Default::default() };
    let res = SearchEngine::execute(&entries, &q);
    assert_eq!(res.entries.len(), 1);
    assert_eq!(res.entries[0].name, "test_folder");
}
