use async_std::task;
use cacache;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tempfile;

fn get(c: &mut Criterion) {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().to_owned();
    let data = b"hello world".to_vec();
    let sri = cacache::put::data(&cache, "hello", data).unwrap();
    c.bench_function("read_hash", move |b| {
        b.iter(|| cacache::get::read_hash(black_box(&cache), black_box(&sri)))
    });
}

fn async_get(c: &mut Criterion) {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().to_owned();
    let data = b"hello world".to_vec();
    let sri = cacache::put::data(&cache, "hello", data).unwrap();
    c.bench_function("async_read_hash", move |b| {
        b.iter(|| {
            task::block_on(cacache::async_get::read_hash(
                black_box(&cache),
                black_box(&sri)
            ))
        })
    });
}

criterion_group!(benches, get, async_get);
criterion_main!(benches);
