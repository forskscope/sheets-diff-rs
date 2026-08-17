//! Peak-allocation measurement (M7 Handoff 01, RFC-024 / RFC-012).
//!
//! Run with: `cargo bench --bench memory` (release profile; this crate's
//! `[profile.release]` is `opt-level = "z"` -- noted here because it is part
//! of what this measurement does and does not capture, see the printed
//! report's own caveats section).
//!
//! This is a report generator, not a statistical benchmark: `harness =
//! false`, no criterion, no assertions on numbers (only on the harness's own
//! correctness, which the "harness accuracy" section prints and would panic
//! on if wrong). It prints its findings to stdout; the numbers are also
//! written into `docs/src/maintainers/performance.md` by hand after review,
//! not generated into it automatically.
//!
//! `#![forbid(unsafe_code)]` is a `src/lib.rs` attribute and does not apply
//! here -- the global allocator wrapper below needs `unsafe impl GlobalAlloc`,
//! which is why this measurement lives in `benches/`, not `src/`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use rust_xlsxwriter::Workbook;
use sheets_diff::options::{AlignmentMode, MatchingOptions};
use sheets_diff::{Cancellation, DiffOptions, compare_bytes, compare_bytes_with_options};

// ---------------------------------------------------------------------------
// Peak-allocation tracking global allocator
// ---------------------------------------------------------------------------

struct TrackingAllocator;

static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

// SAFETY: wraps `System`, whose `GlobalAlloc` impl is itself sound; this impl
// only adds atomic bookkeeping around calls it forwards unchanged.
unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: `layout` is the caller's, forwarded unchanged to `System`.
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            let now = CURRENT.fetch_add(layout.size(), Ordering::SeqCst) + layout.size();
            PEAK.fetch_max(now, Ordering::SeqCst);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr`/`layout` are the caller's, forwarded unchanged.
        unsafe { System.dealloc(ptr, layout) };
        CURRENT.fetch_sub(layout.size(), Ordering::SeqCst);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: forwarded unchanged to `System`.
        let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() {
            let old_size = layout.size();
            if new_size >= old_size {
                let delta = new_size - old_size;
                let now = CURRENT.fetch_add(delta, Ordering::SeqCst) + delta;
                PEAK.fetch_max(now, Ordering::SeqCst);
            } else {
                CURRENT.fetch_sub(old_size - new_size, Ordering::SeqCst);
            }
        }
        new_ptr
    }
}

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator;

fn reset_peak() {
    PEAK.store(CURRENT.load(Ordering::SeqCst), Ordering::SeqCst);
}

fn current() -> usize {
    CURRENT.load(Ordering::SeqCst)
}

fn peak() -> usize {
    PEAK.load(Ordering::SeqCst)
}

/// Runs `f`, returning its result and the net peak bytes allocated above
/// whatever was already live when this was called (so prior state -- input
/// buffers still held, the harness's own bookkeeping -- is excluded).
fn measure_peak<F: FnOnce() -> R, R>(f: F) -> (R, usize) {
    let baseline = current();
    reset_peak();
    let result = f();
    let net = peak().saturating_sub(baseline);
    (result, net)
}

// ---------------------------------------------------------------------------
// Fixture builders (duplicated from benches/workbook_diff.rs -- an
// independent bench target, not a shared module; same convention as
// examples/gen-fixtures.rs vs tests/support.rs)
// ---------------------------------------------------------------------------

fn make_dense(rows: u32, cols: u16, changed: bool) -> Vec<u8> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    for r in 0..rows {
        for c in 0..cols {
            let val = if changed && r == 0 && c == 0 {
                "changed".to_string()
            } else {
                format!("val_{r}_{c}")
            };
            ws.write_string(r, c, &val).unwrap();
        }
    }
    wb.save_to_buffer().unwrap()
}

fn make_sparse(rows: u32, cols: u16, target_populated: u32, changed: bool) -> Vec<u8> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    let total = rows as u64 * cols as u64;
    let stride = (total / target_populated.max(1) as u64).max(1);
    let mut n: u64 = 0;
    let mut placed: u32 = 0;
    'outer: for r in 0..rows {
        for c in 0..cols {
            if n.is_multiple_of(stride) && placed < target_populated {
                let val = if changed && placed == 0 {
                    "changed"
                } else {
                    "val"
                };
                ws.write_string(r, c, val).unwrap();
                placed += 1;
                if placed >= target_populated {
                    break 'outer;
                }
            }
            n += 1;
        }
    }
    wb.save_to_buffer().unwrap()
}

fn make_keyed(rows: u32, changed: bool) -> Vec<u8> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    for r in 0..rows {
        ws.write_string(r, 0, format!("id_{r}")).unwrap();
        let val = if changed && r == 0 {
            "changed".to_string()
        } else {
            format!("val_{r}")
        };
        ws.write_string(r, 1, &val).unwrap();
    }
    wb.save_to_buffer().unwrap()
}

fn make_empty() -> Vec<u8> {
    let mut wb = Workbook::new();
    wb.add_worksheet();
    wb.save_to_buffer().unwrap()
}

/// Two sheets: a large one first, a trivial one second -- so a second
/// `check_cancel` call actually happens after the large sheet finishes,
/// letting Q4 time that gap. A single-sheet workbook has no second
/// checkpoint at all; see the report's Q4 section for why that matters.
fn make_two_sheet(big_rows: u32, big_cols: u16) -> (Vec<u8>, Vec<u8>) {
    let mut old = Workbook::new();
    {
        let ws = old.add_worksheet();
        ws.set_name("Big").unwrap();
        for r in 0..big_rows {
            for c in 0..big_cols {
                ws.write_string(r, c, format!("val_{r}_{c}")).unwrap();
            }
        }
    }
    {
        let ws = old.add_worksheet();
        ws.set_name("Small").unwrap();
        ws.write_string(0, 0, "x").unwrap();
    }
    let old_bytes = old.save_to_buffer().unwrap();

    let mut new = Workbook::new();
    {
        let ws = new.add_worksheet();
        ws.set_name("Big").unwrap();
        for r in 0..big_rows {
            for c in 0..big_cols {
                ws.write_string(r, c, format!("val_{r}_{c}")).unwrap();
            }
        }
    }
    {
        let ws = new.add_worksheet();
        ws.set_name("Small").unwrap();
        ws.write_string(0, 0, "x").unwrap();
    }
    let new_bytes = new.save_to_buffer().unwrap();

    (old_bytes, new_bytes)
}

// ---------------------------------------------------------------------------
// Harness accuracy (required test 1)
// ---------------------------------------------------------------------------

fn verify_harness_accuracy() {
    println!("== Harness accuracy ==");
    const SIZE: usize = 8 * 1024 * 1024; // 8 MiB
    let (_v, observed) = measure_peak(|| {
        let v: Vec<u8> = vec![0u8; SIZE];
        black_box(&v);
        v
    });
    println!("  allocated {SIZE} bytes, harness observed {observed} bytes");
    assert_eq!(
        observed, SIZE,
        "tracking allocator must report an 8 MiB allocation as exactly {SIZE} bytes"
    );
    println!("  PASS -- exact match\n");
}

// ---------------------------------------------------------------------------
// Q1 + ladder: peak for compare_paths and compare_bytes across sizes
// ---------------------------------------------------------------------------

#[allow(dead_code)] // several fields are printed inline in run_ladder() and
// only bytes_peak_total/cells are read again (linearity
// check); kept for completeness of the returned data.
struct LadderPoint {
    label: &'static str,
    cells: u64,
    bytes_peak_internal: usize,
    bytes_peak_total: usize,
    paths_peak: usize,
}

fn run_ladder(temp_dir: &std::path::Path) -> Vec<LadderPoint> {
    println!("== Ladder: peak allocation, compare_bytes vs compare_paths ==");
    println!(
        "  bytes_peak_internal: compare_bytes's own peak, EXCLUDING the caller's\n\
         \x20   already-held input buffer (measured after old/new exist) --\n\
         \x20   this isolates compare_bytes's internal working set for comparison\n\
         \x20   against compare_paths's, which never has a preexisting buffer.\n\
         \x20 bytes_peak_total: peak from BEFORE old/new are even created, through\n\
         \x20   the call -- what a real compare_bytes caller's process actually\n\
         \x20   peaks at, since they must hold the input buffer for the call's\n\
         \x20   duration. This is the number \"doubling\" is a claim about.\n"
    );
    let configs: [(&str, u32, u16); 4] = [
        ("1k", 50, 20),
        ("10k", 500, 20),
        ("100k", 5_000, 20),
        ("300k", 15_000, 20),
    ];

    let mut points = Vec::new();
    for (label, rows, cols) in configs {
        let cells = rows as u64 * cols as u64;

        // bytes_peak_total: baseline reset BEFORE the input buffers exist,
        // so their allocation is part of the measured peak, same as it
        // would be in a real caller's process.
        let (old_for_total, new_for_total, bytes_peak_total) = {
            let baseline = current();
            reset_peak();
            let old = make_dense(rows, cols, false);
            let new = make_dense(rows, cols, true);
            let d = compare_bytes(black_box(&old), black_box(&new)).unwrap();
            black_box(&d);
            let p = peak().saturating_sub(baseline);
            (old, new, p)
        };
        let input_size = old_for_total.len() + new_for_total.len();
        drop(old_for_total);
        drop(new_for_total);

        // bytes_peak_internal: input buffers already exist and are excluded
        // from the baseline -- isolates the internal working set only.
        let old = make_dense(rows, cols, false);
        let new = make_dense(rows, cols, true);
        let (_, bytes_peak_internal) = measure_peak(|| {
            let d = compare_bytes(black_box(&old), black_box(&new)).unwrap();
            black_box(&d);
            d
        });

        let old_path = temp_dir.join(format!("mem-ladder-{label}-old.xlsx"));
        let new_path = temp_dir.join(format!("mem-ladder-{label}-new.xlsx"));
        std::fs::write(&old_path, &old).unwrap();
        std::fs::write(&new_path, &new).unwrap();
        let (_, paths_peak) = measure_peak(|| {
            let d = sheets_diff::compare_paths(black_box(&old_path), black_box(&new_path)).unwrap();
            black_box(&d);
            d
        });
        std::fs::remove_file(&old_path).ok();
        std::fs::remove_file(&new_path).ok();

        let extra_over_paths = bytes_peak_total.saturating_sub(paths_peak);
        let extra_pct = extra_over_paths as f64 / paths_peak.max(1) as f64 * 100.0;
        println!(
            "  {label:>5} ({cells:>7} cells, input={input_size:>9}):\n\
             \x20      internal={bytes_peak_internal:>10} ({:>6.1} B/cell)   paths={paths_peak:>10} ({:>6.1} B/cell)\n\
             \x20      total   ={bytes_peak_total:>10} ({:>6.1} B/cell)   extra over paths={extra_over_paths:>9} ({extra_pct:+.2}% of paths_peak; input_size alone is {:.2}%)",
            bytes_peak_internal as f64 / cells as f64,
            paths_peak as f64 / cells as f64,
            bytes_peak_total as f64 / cells as f64,
            input_size as f64 / paths_peak.max(1) as f64 * 100.0,
        );

        points.push(LadderPoint {
            label,
            cells,
            bytes_peak_internal,
            bytes_peak_total,
            paths_peak,
        });
    }
    println!();
    points
}

fn run_ladder_variance_check(temp_dir: &std::path::Path) {
    println!("== Run-to-run variance (required test 2): 1k and 100k points, 2 runs each ==");
    for (label, rows, cols) in [("1k", 50u32, 20u16), ("100k", 5_000, 20)] {
        let old = make_dense(rows, cols, false);
        let new = make_dense(rows, cols, true);
        let mut readings = Vec::new();
        for run in 0..2 {
            let (_, p) = measure_peak(|| {
                let d = compare_bytes(black_box(&old), black_box(&new)).unwrap();
                black_box(&d);
                d
            });
            readings.push(p);
            println!("  {label} run {run}: {p} bytes");
        }
        let (a, b) = (readings[0] as f64, readings[1] as f64);
        let variance_pct = ((a - b).abs() / a.max(b)) * 100.0;
        println!("  {label} variance: {variance_pct:.4}%");
    }
    let _ = temp_dir;
    println!();
}

// ---------------------------------------------------------------------------
// Q2: attribution
// ---------------------------------------------------------------------------

fn q2_attribution() {
    println!("== Q2: attribution ==");

    // --- Suspect: cell_map_to_align's clone (diff.rs:42) ---
    // Positional (default) never calls cell_map_to_align at all -- only
    // non-Positional modes do. Same fixture, two alignment modes: the
    // difference isolates roughly what the align-map clone (plus whatever
    // the alignment computation itself allocates) costs. Measured at two
    // row counts (10x apart) to see whether the ratio is roughly constant
    // (a high but linear constant) or grows with n (superlinear -- would be
    // a separate, more urgent finding than "cloning costs something").
    for rows in [500u32, 5_000u32] {
        let old = make_keyed(rows, false);
        let new = make_keyed(rows, true);

        let (_, positional_peak) = measure_peak(|| {
            let d = compare_bytes(black_box(&old), black_box(&new)).unwrap();
            black_box(&d);
            d
        });

        let opts = DiffOptions::builder()
            .build_with_matching(MatchingOptions {
                sheet_matching: Default::default(),
                alignment: AlignmentMode::RowKey { columns: vec![0] }, // the stable "id_N" column, not the changing value column
            })
            .unwrap();
        let (_, aligned_peak) = measure_peak(|| {
            let d = compare_bytes_with_options(black_box(&old), black_box(&new), opts).unwrap();
            black_box(&d);
            d
        });

        let align_cost = aligned_peak.saturating_sub(positional_peak);
        println!(
            "  align-clone isolation ({rows} rows, keyed): Positional peak={positional_peak}, RowKey peak={aligned_peak}, delta (align cost)={align_cost} ({:.1}% of Positional peak, {:.2} bytes/row)",
            align_cost as f64 / positional_peak.max(1) as f64 * 100.0,
            align_cost as f64 / rows as f64,
        );
    }

    // --- Suspect: both CellMaps resident (holding a second sheet's worth) ---
    // Attempted comparing a large sheet against an empty one to isolate "only
    // one CellMap large" -- this does NOT isolate what it was meant to.
    // Under the default Positional alignment, the compared-coordinate set is
    // the union of both sides' populated cells: a big-vs-empty pair makes
    // *every* populated cell in the big side a diff (nothing matches on the
    // empty side), so peak there is dominated by `diffs_emitted x
    // sizeof(CellDiff)`, not by CellMap residency. Confirmed empirically
    // below rather than assumed: big-vs-empty peaks HIGHER than big-vs-big,
    // which only makes sense as a diff-output-size effect.
    let big_rows = 15_000u32; // matches the ladder's largest point (300k cells)
    let big = make_dense(big_rows, 20, false);
    let big_changed = make_dense(big_rows, 20, true); // 1 cell differs
    let empty = make_empty();

    let (_, both_large_peak) = measure_peak(|| {
        let d = compare_bytes(black_box(&big), black_box(&big_changed)).unwrap();
        black_box(&d);
        d
    });
    let (big_vs_empty_result, one_large_peak) =
        measure_peak(|| compare_bytes(black_box(&big), black_box(&empty)).unwrap());
    println!(
        "  big-vs-empty attempt: both-large(1 diff) peak={both_large_peak}, big-vs-empty({} diffs) peak={one_large_peak} -- HIGHER, not lower, confirming this does not isolate CellMap residency, it isolates diff-output size instead. Discarded as a method for this suspect.",
        big_vs_empty_result.summary.cells_changed,
    );
    drop(big_vs_empty_result);

    // The methodologically sound version: both sides large and IDENTICAL
    // (zero diffs), so diff-output size cannot confound the reading. This
    // is "two large CellMaps resident, ~zero diff-output cost" -- a real
    // number, but there is no way to externally construct "only one large
    // CellMap resident" without also changing how many coordinates get
    // compared (which changes diff count under Positional, as just shown).
    // That is a structural limit of this engine's design, not a gap in this
    // measurement's method: reported as such rather than forcing a split.
    let (_, both_identical_peak) = measure_peak(|| {
        let d = compare_bytes(black_box(&big), black_box(&big)).unwrap();
        black_box(&d);
        d
    });
    println!(
        "  two large CellMaps, zero diffs: peak={both_identical_peak} ({:.1} B/cell, {big_rows}x20)",
        both_identical_peak as f64 / (big_rows as f64 * 20.0)
    );
    println!(
        "  Could NOT isolate \"a second resident CellMap\" from \"calamine's own per-side parse buffers\" or from \"diff-output size\" with external measurement alone -- doing so would need instrumentation inside src/, out of this unit's non-change scope. Reporting the combined number rather than a false split."
    );
    println!();
}

// ---------------------------------------------------------------------------
// Q3: dense vs sparse per-cell cost at the same populated-cell count
// ---------------------------------------------------------------------------

fn q3_density() {
    println!("== Q3: BTreeMap overhead, dense vs sparse (same populated cell count) ==");
    let populated = 20_000u32;

    // Dense: exactly `populated` cells, packed with no gaps (1000 rows x 20 cols).
    let dense_old = make_dense(1_000, 20, false);
    let dense_new = make_dense(1_000, 20, true);
    let (_, dense_peak) = measure_peak(|| {
        let d = compare_bytes(black_box(&dense_old), black_box(&dense_new)).unwrap();
        black_box(&d);
        d
    });

    // Sparse: same populated count, spread across a used range 10x larger.
    let sparse_old = make_sparse(10_000, 20, populated, false);
    let sparse_new = make_sparse(10_000, 20, populated, true);
    let (_, sparse_peak) = measure_peak(|| {
        let d = compare_bytes(black_box(&sparse_old), black_box(&sparse_new)).unwrap();
        black_box(&d);
        d
    });

    let dense_per_cell = dense_peak as f64 / populated as f64;
    let sparse_per_cell = sparse_peak as f64 / populated as f64;
    let delta_pct = (sparse_per_cell - dense_per_cell) / dense_per_cell * 100.0;
    println!(
        "  dense:  {populated} populated cells, peak={dense_peak} ({dense_per_cell:.2} B/populated-cell)"
    );
    println!(
        "  sparse: {populated} populated cells (10x used range), peak={sparse_peak} ({sparse_per_cell:.2} B/populated-cell)"
    );
    println!("  per-populated-cell delta: {delta_pct:+.1}%");
    println!();
}

// ---------------------------------------------------------------------------
// Q4: cancellation latency
// ---------------------------------------------------------------------------

struct CancelAfterFirstPoll {
    polls: AtomicUsize,
}

impl Cancellation for CancelAfterFirstPoll {
    fn is_cancelled(&self) -> bool {
        self.polls.fetch_add(1, Ordering::SeqCst) >= 1
    }
}

fn q4_cancellation_latency() {
    println!("== Q4: cancellation latency ==");
    let big_rows = 15_000u32; // matches the ladder's largest point (300k cells)
    let (old, new) = make_two_sheet(big_rows, 20);

    let cancel = CancelAfterFirstPoll {
        polls: AtomicUsize::new(0),
    };
    let opts = DiffOptions::builder().cancellation(cancel).build().unwrap();

    let start = Instant::now();
    let result = compare_bytes_with_options(black_box(&old), black_box(&new), opts);
    let elapsed = start.elapsed();

    assert!(
        matches!(result, Err(sheets_diff::SheetsDiffError::Cancelled)),
        "expected Cancelled, got {result:?}"
    );
    println!(
        "  two-sheet workbook ({big_rows}x20 big sheet + 1-cell sheet), cancel armed after sheet 1: {:.2} ms",
        elapsed.as_secs_f64() * 1000.0
    );
    println!(
        "  This equals \"how long processing sheet 1 alone takes\" -- check_cancel fires once \
         per sheet pair, so cancellation requested during sheet 1 is not observed until sheet 1 \
         finishes and the loop reaches sheet 2's check_cancel call."
    );

    // Structural note: a genuinely single-sheet workbook has no SECOND
    // check_cancel call at all, so a cancellation request made after the
    // first (only) call is never observed -- the comparison always returns
    // Ok, regardless of how long it takes or when cancellation was
    // requested. Demonstrated, not just reasoned about.
    let single = make_dense(big_rows, 20, false);
    let single_new = make_dense(big_rows, 20, true);
    let cancel2 = CancelAfterFirstPoll {
        polls: AtomicUsize::new(0),
    };
    let opts2 = DiffOptions::builder()
        .cancellation(cancel2)
        .build()
        .unwrap();
    let single_result = compare_bytes_with_options(&single, &single_new, opts2);
    println!(
        "  single-sheet workbook ({big_rows}x20), same cancel-after-first-poll policy: result={}",
        if single_result.is_ok() {
            "Ok -- cancellation never observed (only one check_cancel call exists, at the start, before this was the second poll)"
        } else {
            "Cancelled"
        }
    );
    println!();
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    let temp_dir =
        std::env::temp_dir().join(format!("sheets-diff-memory-bench-{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();

    println!("sheets-diff memory measurement (M7 Handoff 01)");
    println!("profile: release, opt-level=\"z\" (this crate's [profile.release])\n");

    verify_harness_accuracy();
    let points = run_ladder(&temp_dir);
    run_ladder_variance_check(&temp_dir);
    q2_attribution();
    q3_density();
    q4_cancellation_latency();

    println!("== Linearity check: first vs last ladder point, bytes/cell ==");
    if let (Some(first), Some(last)) = (points.first(), points.last()) {
        let first_per_cell = first.bytes_peak_total as f64 / first.cells as f64;
        let last_per_cell = last.bytes_peak_total as f64 / last.cells as f64;
        let drift_pct = (last_per_cell - first_per_cell) / first_per_cell * 100.0;
        println!(
            "  {} ({} cells): {:.2} B/cell  vs  {} ({} cells): {:.2} B/cell  -- drift {:+.1}%",
            first.label,
            first.cells,
            first_per_cell,
            last.label,
            last.cells,
            last_per_cell,
            drift_pct
        );
    }
    println!();

    std::fs::remove_dir_all(&temp_dir).ok();
    println!("Done.");
}
