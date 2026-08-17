//! v1.2-vs-v2 benchmark comparison (M7 Handoff 02, RFC-027).
//!
//! Run with: `cargo bench --bench v1_2_comparison` (release profile; see
//! `docs/src/maintainers/performance.md` for this crate's `opt-level = "z"`
//! caveat, which applies here identically to `benches/memory.rs`).
//!
//! **Commit measured:** see `docs/src/maintainers/performance.md`'s v1.2
//! comparison section for the exact commit this report was generated
//! against (must be at or after `db88706`, M7 unit 03's merge — see that
//! section for why this matters).
//!
//! This is a report generator, not a statistical benchmark: `harness =
//! false`, no criterion for the cross-version comparison (criterion's own
//! harness is used for `benches/workbook_diff.rs`'s intra-version
//! benchmarks; running two independently-versioned crates through one
//! criterion group is not something criterion is designed for, and
//! reinventing that plumbing here would be scope this unit doesn't need).
//! No performance threshold in CI — same reasoning as `benches/memory.rs`.
//!
//! **Four confounds, and how each is handled — read this before the
//! numbers below:**
//!
//! 1. **The dependency differs.** v1.2.0 pins `calamine = "0"` (loose);
//!    with no lockfile entry forcing 0.35.0 specifically, Cargo's resolver
//!    unifies both v1.2 and v2 onto the **same** calamine 0.36.1 in this
//!    workspace's `Cargo.lock` (confirmed with `cargo tree -i calamine`,
//!    not assumed) — **this confound is eliminated by construction, not
//!    bounded**, which is a stronger resolution than a separate
//!    0.35-vs-0.36 measurement would have given. This is different from
//!    what the handoff anticipated (it expected v1.2's own lockfile-pinned
//!    0.35.0 and asked for that difference to be controlled or bounded);
//!    deliberately not pinning calamine down to 0.35.0 for v1.2 is what
//!    makes the elimination hold, so this is a choice, not an oversight.
//! 2. **They don't do the same work.** v1.2 compares cells as strings; v2
//!    normalises into typed `CellValue`s, aligns rows, produces
//!    diagnostics, and tracks metrics. Stated next to every number below,
//!    not once at the top and forgotten.
//! 3. **v1.2 has no benchmarks of its own.** This harness drives both
//!    through their path-based entry points — `Diff::try_new` (v1.2) and
//!    `compare_paths` (v2) — the only entry point shape both versions
//!    share.
//! 4. **v2's polling counter (M7 Handoff 03).** v1.2 has no cancellation
//!    mechanism at all, so no equivalent cost exists to compare against.
//!    Handoff 03's own measurement (`performance.md`, Q5) found this
//!    overhead below run-to-run noise at every ladder size; cited, not
//!    re-measured or subtracted here — doing so would be false precision
//!    smaller than this harness's own timing variance.

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use rust_xlsxwriter::Workbook;

// ---------------------------------------------------------------------------
// Peak-allocation tracking global allocator
//
// Duplicated from benches/memory.rs rather than shared -- each bench target
// is its own binary, and only one #[global_allocator] can be set per binary.
// Same convention as this project's fixture-builder duplication across
// bench/example/test files.
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

fn measure_peak<F: FnOnce() -> R, R>(f: F) -> (R, usize) {
    let baseline = current();
    reset_peak();
    let result = f();
    let net = peak().saturating_sub(baseline);
    (result, net)
}

// ---------------------------------------------------------------------------
// Fixture builders (adapted from benches/workbook_diff.rs's shapes)
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

fn make_sparse(rows: u32, cols: u16, density_pct: u32, changed: bool) -> Vec<u8> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    let mut n: u32 = 0;
    for r in 0..rows {
        for c in 0..cols {
            n += 1;
            if n.is_multiple_of(100 / density_pct) {
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

// ---------------------------------------------------------------------------
// Agreement check (required test 1): both versions must count the same
// number of changed cells on an unambiguous, value-only-change fixture.
// v1.2 emits up to TWO CellDiff entries per address (one Value, one
// Formula, per its own CellDiffKind); a value-only fixture keeps its total
// cell_diffs.len() directly comparable to v2's per-address cells_changed.
// ---------------------------------------------------------------------------

fn run_agreement_check(temp_dir: &std::path::Path) {
    println!("== Agreement check (required test 1) ==");
    let old = make_dense(20, 5, false);
    let new = make_dense(20, 5, true); // exactly 1 cell differs: (0,0)

    let old_path = temp_dir.join("agree-old.xlsx");
    let new_path = temp_dir.join("agree-new.xlsx");
    std::fs::write(&old_path, &old).unwrap();
    std::fs::write(&new_path, &new).unwrap();

    let v1_diff = sheets_diff_v1_2::core::diff::Diff::try_new(&old_path, &new_path)
        .expect("v1.2 try_new should succeed on a well-formed fixture");
    let v1_changed: usize = v1_diff.cell_diffs.iter().map(|sc| sc.cells.len()).sum();

    let v2_diff =
        sheets_diff::compare_paths(&old_path, &new_path).expect("v2 compare_paths should succeed");
    let v2_changed = v2_diff.summary.cells_changed;

    std::fs::remove_file(&old_path).ok();
    std::fs::remove_file(&new_path).ok();

    println!("  v1.2 cell_diffs (summed across sheets): {v1_changed}");
    println!("  v2   summary.cells_changed:             {v2_changed}");
    assert_eq!(
        v1_changed, v2_changed as usize,
        "v1.2 and v2 disagree on changed-cell count for an unambiguous fixture -- \
         a benchmark of two functions that disagree about the answer is not a comparison"
    );
    println!("  PASS -- both versions agree\n");
}

// ---------------------------------------------------------------------------
// Timing + memory comparison, per scenario
// ---------------------------------------------------------------------------

struct ScenarioResult {
    name: &'static str,
    v1_ms: Vec<f64>,
    v2_ms: Vec<f64>,
    v1_peak: usize,
    v2_peak: usize,
}

fn run_scenario(
    name: &'static str,
    old: Vec<u8>,
    new: Vec<u8>,
    temp_dir: &std::path::Path,
    repeats: u32,
) -> ScenarioResult {
    let old_path = temp_dir.join(format!("{name}-old.xlsx"));
    let new_path = temp_dir.join(format!("{name}-new.xlsx"));
    std::fs::write(&old_path, &old).unwrap();
    std::fs::write(&new_path, &new).unwrap();

    let mut v1_ms = Vec::with_capacity(repeats as usize);
    let mut v2_ms = Vec::with_capacity(repeats as usize);

    for _ in 0..repeats {
        let start = Instant::now();
        let d =
            sheets_diff_v1_2::core::diff::Diff::try_new(black_box(&old_path), black_box(&new_path))
                .unwrap();
        black_box(&d);
        v1_ms.push(start.elapsed().as_secs_f64() * 1000.0);

        let start = Instant::now();
        let d = sheets_diff::compare_paths(black_box(&old_path), black_box(&new_path)).unwrap();
        black_box(&d);
        v2_ms.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    let (_, v1_peak) = measure_peak(|| {
        let d =
            sheets_diff_v1_2::core::diff::Diff::try_new(black_box(&old_path), black_box(&new_path))
                .unwrap();
        black_box(&d);
        d
    });
    let (_, v2_peak) = measure_peak(|| {
        let d = sheets_diff::compare_paths(black_box(&old_path), black_box(&new_path)).unwrap();
        black_box(&d);
        d
    });

    std::fs::remove_file(&old_path).ok();
    std::fs::remove_file(&new_path).ok();

    ScenarioResult {
        name,
        v1_ms,
        v2_ms,
        v1_peak,
        v2_peak,
    }
}

fn avg(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

fn variance_pct(v: &[f64]) -> f64 {
    let (min, max) = v
        .iter()
        .fold((f64::MAX, f64::MIN), |(mn, mx), &x| (mn.min(x), mx.max(x)));
    (max - min) / max.max(1e-9) * 100.0
}

fn print_scenario(r: &ScenarioResult) {
    let v1_avg = avg(&r.v1_ms);
    let v2_avg = avg(&r.v2_ms);
    let time_delta_pct = (v2_avg - v1_avg) / v1_avg.max(1e-9) * 100.0;
    let mem_delta_pct = (r.v2_peak as f64 - r.v1_peak as f64) / (r.v1_peak.max(1) as f64) * 100.0;

    println!("-- {} --", r.name);
    println!(
        "  time: v1.2 avg={:.3} ms {:?} (variance {:.2}%)",
        v1_avg,
        r.v1_ms
            .iter()
            .map(|v| format!("{v:.2}"))
            .collect::<Vec<_>>(),
        variance_pct(&r.v1_ms)
    );
    println!(
        "        v2   avg={:.3} ms {:?} (variance {:.2}%)",
        v2_avg,
        r.v2_ms
            .iter()
            .map(|v| format!("{v:.2}"))
            .collect::<Vec<_>>(),
        variance_pct(&r.v2_ms)
    );
    println!("        v2 vs v1.2: {time_delta_pct:+.1}%");
    println!(
        "  peak memory: v1.2={} bytes   v2={} bytes   v2 vs v1.2: {mem_delta_pct:+.1}%",
        r.v1_peak, r.v2_peak
    );
    println!();
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    let temp_dir =
        std::env::temp_dir().join(format!("sheets-diff-v1-2-bench-{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();

    println!("sheets-diff v1.2-vs-v2 comparison (M7 Handoff 02)");
    println!("profile: release, opt-level=\"z\" (this crate's [profile.release])");
    println!("v1.2 pin: sheets-diff-v1_2 = \"=1.2.0\" (Cargo.toml)");
    println!(
        "v2 version under test: {} -- see docs/src/maintainers/performance.md \
         for the exact commit this report's numbers were generated against",
        env!("CARGO_PKG_VERSION")
    );
    println!();

    run_agreement_check(&temp_dir);

    const REPEATS: u32 = 5;
    let scenarios: Vec<ScenarioResult> = vec![
        run_scenario(
            "small_dense",
            make_dense(50, 20, false),
            make_dense(50, 20, true),
            &temp_dir,
            REPEATS,
        ),
        run_scenario(
            "tall",
            make_dense(5_000, 20, false),
            make_dense(5_000, 20, true),
            &temp_dir,
            REPEATS,
        ),
        run_scenario(
            "sparse",
            make_sparse(1_000, 100, 5, false),
            make_sparse(1_000, 100, 5, true),
            &temp_dir,
            REPEATS,
        ),
    ];

    println!("== Per-scenario results ==");
    println!(
        "What v2 does that v1.2 does not, for every number below: typed \
         CellValue normalisation (not string comparison), row alignment \
         support, per-cell diagnostics, DiffMetrics tracking, and \
         independent value/formula change detection. v2 being slower is \
         not automatically a defect -- it may be the cost of capability \
         v1.2 never had.\n"
    );
    for r in &scenarios {
        print_scenario(r);
    }

    std::fs::remove_dir_all(&temp_dir).ok();
    println!("Done.");
}
