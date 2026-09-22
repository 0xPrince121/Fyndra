use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use fyndra_core::{CompactEntry, DirectoryStore, SearchEngine, SearchQuery};
use std::path::Path;

fn bench_search(c: &mut Criterion) {
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
    let mut group = c.benchmark_group("search_100k");
    for query in ["worker_99", "module", "nonexistent_xyz"] {
        group.bench_with_input(BenchmarkId::from_parameter(query), &query, |b, q| {
            let query_obj = SearchQuery { raw: q.to_string(), ..Default::default() };
            b.iter(|| {
                let res = SearchEngine::execute_compact(
                    black_box(&entries),
                    black_box(&dirs),
                    black_box(&query_obj),
                );
                black_box(res.entries.len());
            });
        });
    }
    group.finish();
}
criterion_group!(benches, bench_search);
criterion_main!(benches);
