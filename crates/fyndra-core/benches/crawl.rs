use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fyndra_core::{Crawler, CrawlerConfig};
use std::fs;
use tempfile::TempDir;

fn bench_crawl(c: &mut Criterion) {
    // Create temp tree with 5k files
    let tmp = TempDir::new().unwrap();
    for i in 0..50 {
        let dir = tmp.path().join(format!("dir_{:02}", i));
        fs::create_dir_all(&dir).unwrap();
        for j in 0..100 {
            fs::write(dir.join(format!("file_{:03}.txt", j)), b"hello").unwrap();
        }
    }
    let crawler = Crawler::new(CrawlerConfig::default());
    let roots = vec![tmp.path().to_path_buf()];
    c.bench_function("crawl_5k_files", |b| {
        b.iter(|| {
            let (entries, _dirs, stats) = crawler.crawl_roots_compact(black_box(&roots));
            black_box((entries.len(), stats.total_files));
        })
    });
}
criterion_group!(benches, bench_crawl);
criterion_main!(benches);
