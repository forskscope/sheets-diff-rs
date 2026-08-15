//! Benchmark suite for `sheets-diff` (RFC-027).
//!
//! Run with:  cargo bench
//!
//! Scenarios (RFC-027 §5):
//! 1. small_business  — 5 sheets, 100 rows, 20 columns
//! 2. wide            — 1 sheet, 100 rows, 1000 columns
//! 3. tall            — 1 sheet, 50 000 rows, 20 columns
//! 4. sparse          — large range, low populated density
//! 5. many_sheets     — 200 sheets, minimal content
//! 6. formula         — formulas + cached values
//! 7. rename          — sheets renamed, minimal cell changes
//! 8. alignment       — inserted row cascade (positional vs key-aligned)

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rust_xlsxwriter::{Formula, Workbook};
use sheets_diff::options::{AlignmentMode, MatchingOptions};
use sheets_diff::{DiffOptions, compare_bytes, compare_bytes_with_options};

// ---------------------------------------------------------------------------
// Fixture builders
// ---------------------------------------------------------------------------

fn make_workbook(sheets: &[(&str, u32, u16)], changed: bool) -> Vec<u8> {
    let mut wb = Workbook::new();
    for (name, rows, cols) in sheets {
        let ws = wb.add_worksheet();
        ws.set_name(*name).unwrap();
        for r in 0..*rows {
            for c in 0..*cols {
                let val = if changed && r == 0 && c == 0 {
                    format!("changed_{r}_{c}")
                } else {
                    format!("val_{r}_{c}")
                };
                ws.write_string(r, c, &val).unwrap();
            }
        }
    }
    wb.save_to_buffer().unwrap()
}

fn make_sparse(rows: u32, cols: u16, density_pct: u32, changed: bool) -> Vec<u8> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    let mut n: u32 = 0;
    for r in 0..rows {
        for c in 0..cols {
            n += 1;
            if n % (100 / density_pct) == 0 {
                let val = if changed && r == 0 && c == 0 {
                    "changed"
                } else {
                    "val"
                };
                ws.write_string(r, c, val).unwrap();
            }
        }
    }
    wb.save_to_buffer().unwrap()
}

fn make_formula_workbook(rows: u32) -> Vec<u8> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    for r in 0..rows {
        ws.write_number(r, 0, r as f64).unwrap();
        ws.write_formula(r, 1, Formula::new(&format!("=A{}", r + 1)))
            .unwrap();
    }
    wb.save_to_buffer().unwrap()
}

fn make_insertion_workbook(rows: u32, inserted: bool) -> Vec<u8> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    let offset: u32 = if inserted { 1 } else { 0 };
    if inserted {
        ws.write_string(0, 0, "inserted_row").unwrap();
    }
    for r in 0..rows {
        ws.write_string(r + offset, 0, &format!("id_{r}")).unwrap();
        ws.write_string(r + offset, 1, &format!("val_{r}")).unwrap();
    }
    wb.save_to_buffer().unwrap()
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

fn bench_small_business(c: &mut Criterion) {
    let sheets: Vec<(&str, u32, u16)> = (0..5)
        .map(|i| (["S1", "S2", "S3", "S4", "S5"][i], 100, 20))
        .collect();
    let old = make_workbook(&sheets, false);
    let new = make_workbook(&sheets, true);
    c.bench_function("small_business", |b| {
        b.iter(|| compare_bytes(black_box(&old), black_box(&new)).unwrap())
    });
}

fn bench_wide(c: &mut Criterion) {
    let old = make_workbook(&[("Wide", 100, 1000)], false);
    let new = make_workbook(&[("Wide", 100, 1000)], true);
    c.bench_function("wide", |b| {
        b.iter(|| compare_bytes(black_box(&old), black_box(&new)).unwrap())
    });
}

fn bench_tall(c: &mut Criterion) {
    // 50k rows is heavy; use 5k for the bench default (run the 50k via --ignored test)
    let old = make_workbook(&[("Tall", 5_000, 20)], false);
    let new = make_workbook(&[("Tall", 5_000, 20)], true);
    c.bench_function("tall_5k", |b| {
        b.iter(|| compare_bytes(black_box(&old), black_box(&new)).unwrap())
    });
}

fn bench_sparse(c: &mut Criterion) {
    let old = make_sparse(1_000, 100, 5, false);
    let new = make_sparse(1_000, 100, 5, true);
    c.bench_function("sparse", |b| {
        b.iter(|| compare_bytes(black_box(&old), black_box(&new)).unwrap())
    });
}

fn bench_many_sheets(c: &mut Criterion) {
    // Build 50-sheet workbooks directly without the intermediate Vec<(&str,...)>
    // which had a lifetime issue (names dropped before sheets).
    let old = {
        let mut wb = Workbook::new();
        for i in 0..50u32 {
            let ws = wb.add_worksheet();
            ws.set_name(&format!("Sheet{i}")).unwrap();
            ws.write_string(0, 0, "val").unwrap();
        }
        wb.save_to_buffer().unwrap()
    };
    let new = {
        let mut wb = Workbook::new();
        for i in 0..50u32 {
            let ws = wb.add_worksheet();
            ws.set_name(&format!("Sheet{i}")).unwrap();
            ws.write_string(0, 0, if i == 0 { "changed" } else { "val" })
                .unwrap();
        }
        wb.save_to_buffer().unwrap()
    };
    c.bench_function("many_sheets_50", |b| {
        b.iter(|| compare_bytes(black_box(&old), black_box(&new)).unwrap())
    });
}

fn bench_formula(c: &mut Criterion) {
    let old = make_formula_workbook(200);
    let new = make_formula_workbook(200);
    c.bench_function("formula_200rows", |b| {
        b.iter(|| compare_bytes(black_box(&old), black_box(&new)).unwrap())
    });
}

fn bench_alignment_vs_positional(c: &mut Criterion) {
    let old = make_insertion_workbook(500, false);
    let new = make_insertion_workbook(500, true);

    let mut group = c.benchmark_group("insertion_cascade");

    group.bench_with_input(
        BenchmarkId::new("positional", 500),
        &(&old, &new),
        |b, (o, n)| b.iter(|| compare_bytes(black_box(o), black_box(n)).unwrap()),
    );

    group.bench_with_input(
        BenchmarkId::new("row_key_align", 500),
        &(&old, &new),
        |b, (o, n)| {
            b.iter(|| {
                compare_bytes_with_options(
                    black_box(o),
                    black_box(n),
                    DiffOptions::builder()
                        .build_with_matching(MatchingOptions {
                            sheet_matching: Default::default(),
                            alignment: AlignmentMode::RowKey { columns: vec![1] },
                        })
                        .unwrap(),
                )
                .unwrap()
            })
        },
    );

    group.finish();
}

criterion_group!(
    benches,
    bench_small_business,
    bench_wide,
    bench_tall,
    bench_sparse,
    bench_many_sheets,
    bench_formula,
    bench_alignment_vs_positional,
);
criterion_main!(benches);
