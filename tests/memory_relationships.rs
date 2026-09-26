//! Relationships between memory peaks that hold on any machine (M9 unit 00).
//!
//! `benches/memory.rs` is a report generator and asserts nothing about numbers: its figures are one
//! machine's. What *is* portable is a relationship between two peaks measured in the same process,
//! or between a peak and something the algorithm defines. This file holds those.
//! `tests/streaming_read.rs` already guards the other portable property (a sparse sheet's read
//! stays under 64 MiB, a threshold with two orders of magnitude on either side).
//!
//! **What is guarded: choosing an alignment mode costs the alignment table and nothing else.**
//! `RowKey` and `RowSignature` run an LCS over the two sequences of rows, which allocates a
//! `(m + 1) x (n + 1)` table of `u32` — that is what `Limits::max_alignment_product` bounds, and it
//! is quadratic in rows by design. M7 handoff 04 deleted a second cost: `cell_map_to_align` cloned
//! every cell value into an owned map so that `align.rs` could call `display_string()` on two
//! fields of it, +33% of peak, linear in cells. The fixture here has few rows and heavy cells, so
//! the table is small and a clone of the cells is large: reintroducing an owned copy of the cell
//! values fails the first two tests by a wide margin, and nothing else does.
//!
//! **The assertions are relationships, never a byte count**: the aligned peak against the
//! `Positional` peak from the *same fixture in the same process*, plus the table's size as a
//! function of the row counts. The absolute numbers vary with allocator and platform; these do not.
//!
//! ## What the M7 measurement did not show
//!
//! `performance.md` reported that after the clone was deleted the `RowKey` peak *equalled* the
//! `Positional` peak, "delta exactly 0". That was measured with `columns: vec![0]`, and key columns
//! are **1-based**: column 0 selects no cell, so no row had a key and the LCS ran on two empty
//! sequences. It measured the absence of the clone and nothing about alignment. With the key column
//! actually populated the delta is the table above (about 100 MB at 5,000 rows, the default bound).
//! The first test's `matched_rows == rows` assertion is what stops this file repeating that: it
//! fails if the alignment did not really run.

mod support;

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use rust_xlsxwriter::Workbook;
use sheets_diff::options::AlignmentMode;
use sheets_diff::{DiffOptions, MatchConfidence, compare_bytes_with_options};

struct Counting;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

// SAFETY: defers every operation to `System`; only counts.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            let now = LIVE.fetch_add(layout.size(), Ordering::SeqCst) + layout.size();
            PEAK.fetch_max(now, Ordering::SeqCst);
        }
        p
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        LIVE.fetch_sub(layout.size(), Ordering::SeqCst);
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let p = unsafe { System.realloc(ptr, layout, new_size) };
        if !p.is_null() {
            let now = if new_size >= layout.size() {
                LIVE.fetch_add(new_size - layout.size(), Ordering::SeqCst) + new_size
                    - layout.size()
            } else {
                LIVE.fetch_sub(layout.size() - new_size, Ordering::SeqCst)
                    - (layout.size() - new_size)
            };
            PEAK.fetch_max(now, Ordering::SeqCst);
        }
        p
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

/// The tests share one allocator and one peak counter, so they run one at a time.
static SERIAL: Mutex<()> = Mutex::new(());

/// Peak live heap growth, in bytes, while `f` runs.
fn peak_growth<T>(f: impl FnOnce() -> T) -> (T, usize) {
    let base = LIVE.load(Ordering::SeqCst);
    PEAK.store(base, Ordering::SeqCst);
    let out = f();
    let peak = PEAK.load(Ordering::SeqCst).saturating_sub(base);
    eprintln!("[peak] {peak} bytes");
    (out, peak)
}

/// Few rows, heavy cells: `ROWS` rows, column A a unique id, `HEAVY_COLS` further columns of
/// `CELL_CHARS`-character text. The LCS table is `(ROWS + 1)^2` `u32` — about 40 KB — against
/// roughly a megabyte of cell text a side, so an owned copy of the cells would dwarf it.
const ROWS: u32 = 100;
const HEAVY_COLS: u16 = 40;
const CELL_CHARS: usize = 200;

fn heavy_sheet(changed: bool) -> Vec<u8> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    for r in 0..ROWS {
        ws.write_string(r, 0, format!("id_{r}")).unwrap();
        for c in 1..=HEAVY_COLS {
            let mut text = format!("r{r}c{c}-");
            text.extend(std::iter::repeat_n('x', CELL_CHARS));
            if changed && r == 0 && c == 1 {
                text.push('!');
            }
            ws.write_string(r, c, &text).unwrap();
        }
    }
    wb.save_to_buffer().unwrap()
}

/// Bytes of the LCS table an alignment of two `rows`-row sequences allocates: `(m + 1) x (n + 1)`
/// `u32`, plus one `Vec` header per table row. This is the quantity `max_alignment_product` bounds
/// (it counts `m x n`), and it is defined by the algorithm, not measured.
fn lcs_table_bytes(rows: u32) -> usize {
    let side = rows as usize + 1;
    side * side * size_of::<u32>() + side * size_of::<Vec<u32>>()
}

fn opts(mode: AlignmentMode) -> DiffOptions {
    DiffOptions::builder().alignment(mode).build().unwrap()
}

/// Peak of comparing the heavy pair under `mode`, and the alignment summary it produced.
fn measure(mode: AlignmentMode) -> (usize, Option<(usize, MatchConfidence)>) {
    let (old, new) = (heavy_sheet(false), heavy_sheet(true));
    let (diff, peak) = peak_growth(|| compare_bytes_with_options(&old, &new, opts(mode)).unwrap());
    let a = diff.sheets[0]
        .alignment_summary
        .as_ref()
        .map(|a| (a.matched_rows, a.confidence));
    (peak, a)
}

/// Headroom over `Positional + table`: 5% of the `Positional` peak. The keys, the two `BTreeMap`s of
/// them and the mapping are a few kilobytes here; an owned copy of the cells is over a megabyte.
fn allowance(positional_peak: usize) -> usize {
    positional_peak / 20
}

#[test]
fn row_key_costs_the_alignment_table_and_no_copy_of_the_cells() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (positional, none) = measure(AlignmentMode::Positional);
    assert!(none.is_none(), "Positional has no alignment summary");

    let (aligned, summary) = measure(AlignmentMode::RowKey { columns: vec![1] });
    // The guard against measuring nothing: the key column is populated, every row is matched, and
    // so the alignment — LCS table included — really ran.
    assert_eq!(
        summary,
        Some((ROWS as usize, MatchConfidence::Exact)),
        "the fixture must exercise the alignment: 1-based column 1 is the id column"
    );

    let budget = positional + lcs_table_bytes(ROWS) + allowance(positional);
    assert!(
        aligned <= budget,
        "RowKey peak {aligned} exceeds Positional {positional} + LCS table {} + 5% ({}): \
         alignment is holding something besides the table — an owned copy of the cell values?",
        lcs_table_bytes(ROWS),
        allowance(positional)
    );
}

/// The same relationship at many rows and small cells, for both modes that run an LCS. Here the
/// table is the dominant term (about 9 MB against a few hundred kilobytes for everything else), so
/// the bound can be tight in relative terms, and the *lower* bound below proves the table is
/// really there — a bound that only passed because the alignment had not run would not be one.
///
/// (This does not detect an owned copy of the cells — at this cell size a copy is small beside the
/// table, and a signature is itself about as large as the cell text. The heavy fixture above is
/// the guard for that, on `RowKey`, whose only per-row cost is the key.)
#[test]
fn the_table_dominates_and_nothing_else_grows_with_rows() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let rows = 1_500u32;
    let make = |changed: bool| {
        let mut wb = Workbook::new();
        let ws = wb.add_worksheet();
        for r in 0..rows {
            ws.write_string(r, 0, format!("id_{r}")).unwrap();
            let v = if changed && r == 0 {
                "changed".to_string()
            } else {
                format!("val_{r}")
            };
            ws.write_string(r, 1, &v).unwrap();
        }
        wb.save_to_buffer().unwrap()
    };
    let (old, new) = (make(false), make(true));

    let (_, positional) =
        peak_growth(|| compare_bytes_with_options(&old, &new, DiffOptions::default()).unwrap());
    let table = lcs_table_bytes(rows);

    for (label, mode, matched) in [
        (
            "RowKey",
            AlignmentMode::RowKey { columns: vec![1] },
            rows as usize,
        ),
        (
            "RowSignature",
            AlignmentMode::RowSignature {
                sample_columns: None,
            },
            rows as usize - 1, // one changed cell changes one row's signature
        ),
    ] {
        let (diff, aligned) =
            peak_growth(|| compare_bytes_with_options(&old, &new, opts(mode)).unwrap());
        assert_eq!(
            diff.sheets[0]
                .alignment_summary
                .as_ref()
                .unwrap()
                .matched_rows,
            matched,
            "{label}: the alignment must really run"
        );
        assert!(
            aligned <= positional + table + table / 20 + allowance(positional),
            "{label}: peak {aligned} vs Positional {positional} + LCS table {table}"
        );
        assert!(
            aligned > positional + table / 2,
            "{label}: the LCS table should be visible in the peak ({aligned} vs {positional} + \
             {table}): the alignment did not run, or the table was replaced — revisit this file \
             and performance.md"
        );
    }
}
