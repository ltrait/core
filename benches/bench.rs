#[path = "../tests/dummyui.rs"]
mod dummyui;

use color_eyre::eyre::Result;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

use dummyui::DummyUI;
use ltrait::{Launcher, filter::ClosureFilter, sorter::ClosureSorter, source::from_iter};

use std::convert::identity;

use tokio::runtime::Runtime;

async fn simple_source_a() -> Result<()> {
    let launcher = Launcher::default()
        .add_source(from_iter(0..black_box(500_000)), identity)
        .batch_size(10_000)
        .set_ui(DummyUI::new(|_: &()| {}), |_| ());

    launcher.run().await?;

    Ok(())
}

async fn simple_filter_a() -> Result<()> {
    let launcher = Launcher::default()
        .add_source(from_iter(0..black_box(500_000)), identity)
        .batch_size(10_000)
        .add_filter(
            ClosureFilter::new(|_: &i32, _| black_box(true)),
            |&c: &i32| c,
        )
        .add_filter(
            ClosureFilter::new(|_: &i32, _| black_box(true)),
            |&c: &i32| c,
        )
        .add_filter(
            ClosureFilter::new(|_: &i32, _| black_box(true)),
            |&c: &i32| c,
        )
        .set_ui(DummyUI::new(|_: &()| {}), |_| ());

    launcher.run().await?;

    Ok(())
}

async fn simple_sorter_a() -> Result<()> {
    let launcher = Launcher::default()
        .add_source(from_iter(0..black_box(500_000)), identity)
        .batch_size(10_000)
        .add_sorter(
            ClosureSorter::new(|_: &i32, _: &i32, _| black_box(std::cmp::Ordering::Equal)),
            |&c: &i32| c,
        )
        .add_sorter(
            ClosureSorter::new(|_: &i32, _: &i32, _| black_box(std::cmp::Ordering::Equal)),
            |&c: &i32| c,
        )
        .add_sorter(
            ClosureSorter::new(|_: &i32, _: &i32, _| black_box(std::cmp::Ordering::Equal)),
            |&c: &i32| c,
        )
        .set_ui(DummyUI::new(|_: &()| {}), |_| ());

    launcher.run().await?;

    Ok(())
}

async fn three_sources_a() -> Result<()> {
    let launcher = Launcher::default()
        .add_source(from_iter(0..black_box(150_000)), identity)
        .add_source(from_iter(0..black_box(150_000)), identity)
        .add_source(from_iter(0..black_box(150_000)), identity)
        .batch_size(10_000)
        .set_ui(DummyUI::new(|_: &()| {}), |_| ());

    launcher.run().await?;

    Ok(())
}

fn simple_source(c: &mut Criterion) {
    c.bench_function("500,000 Items, batch_size = 10,000", |b| {
        b.to_async(Runtime::new().unwrap())
            .iter(|| simple_source_a());
    });
}

fn simple_filter(c: &mut Criterion) {
    c.bench_function("500,000 Items, batch_size = 10,000, 3 filters", |b| {
        b.to_async(Runtime::new().unwrap())
            .iter(|| simple_filter_a());
    });
}

fn simple_sorter(c: &mut Criterion) {
    c.bench_function("500,000 Items, batch_size = 10,000, 3 sorters", |b| {
        b.to_async(Runtime::new().unwrap())
            .iter(|| simple_sorter_a());
    });
}

fn three_sources(c: &mut Criterion) {
    c.bench_function("450,000 Items, batch_size = 10,000, 3 sources", |b| {
        b.to_async(Runtime::new().unwrap())
            .iter(|| three_sources_a());
    });
}

criterion_group!(
    benches,
    simple_source,
    simple_filter,
    simple_sorter,
    three_sources,
);
criterion_main!(benches);
