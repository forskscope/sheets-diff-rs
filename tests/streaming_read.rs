//! The sheet reader spends memory in proportion to the *populated* cells, and
//! checks its bounds and the cancellation flag inside the loop that spends it.
//!
//! `Xlsx::worksheet_range` returns a dense range: rows x columns of the bounding
//! box of the populated cells. One stray cell far from the data therefore made a
//! workbook of a few kilobytes allocate gigabytes, and neither `max_cells_read`
//! nor the cancellation poll could run first, because both lived in a loop over
//! the finished range. These tests measure that directly.
//!
//! **What is measured is memory, not time.** A test that asserts "under N
//! seconds" is a property of the machine it ran on. Peak live heap bytes, taken
//! from a counting global allocator, are the same on every machine (to within a
//! few bytes: the zip embeds a timestamp). The threshold (64 MiB = 67,108,864
//! bytes) sits above the streamed read and below the dense one, by these
//! measurements (2026-09-24; the dense figures are the pre-fix code, `7dc12f7^`):
//!
//! | fixture | dense (pre-fix) | x threshold | streamed |
//! |---|---|---|---|
//! | stray cell, stray formula, `max_cells_read` | ~646 MB | ~9.6 | ~0.13 MB |
//! | 60,000 cells + far stray cell (cancellation) | ~1.29 GB | ~19.3 | ~12.5 MB |
//!
//! So the threshold is a wide margin on both sides, not an extreme one: neither a
//! noisy allocator nor a different platform moves a test across it.
//!
//! The tests share one allocator and one peak counter, so they run one at a time
//! under `SERIAL`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use rust_xlsxwriter::{Formula, Workbook};
use sheets_diff::{
    Cancellation, DiffOptions, LimitKind, Limits, SheetsDiffError, compare_bytes,
    compare_bytes_with_options,
};

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

const BUDGET: usize = 64 * 1024 * 1024;

/// Far row and column of the stray cell. The bounding box it makes with A1 is
/// 200,001 x 101 = 20,200,101 positions per side: a dense read of that is about
/// 646 MB on this crate's own types (32 bytes a position), and a workbook that
/// produces it is about 5 KB.
const FAR_ROW: u32 = 200_000;
const FAR_COL: u16 = 100;
const FAR_AREA: u64 = (FAR_ROW as u64 + 1) * (FAR_COL as u64 + 1);

/// A1 plus one stray cell at (`FAR_ROW`, `FAR_COL`).
fn wb_stray(stray: &str) -> Vec<u8> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.write_string(0, 0, "a").unwrap();
    ws.write_string(FAR_ROW, FAR_COL, stray).unwrap();
    wb.save_to_buffer().unwrap()
}

#[test]
fn a_stray_far_cell_is_read_in_memory_proportional_to_populated_cells() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (old, new) = (wb_stray("x"), wb_stray("y"));
    assert!(
        old.len() < 16 * 1024,
        "fixture is meant to be tiny, got {} bytes",
        old.len()
    );

    let (diff, peak) = peak_growth(|| compare_bytes(&old, &new).unwrap());

    assert_eq!(
        diff.sheets[0].cell_diffs.len(),
        1,
        "the stray cell's change is reported"
    );
    assert!(
        peak < BUDGET,
        "peak heap growth {peak} bytes for a {}-byte workbook with two populated \
         cells: memory must follow the populated cells, not the bounding box",
        old.len()
    );
}

#[test]
fn cells_read_still_counts_the_bounding_box() {
    // The metric and the limit keep their meaning (a sheet contributes the
    // area of the bounding box of its populated cells); only when it is
    // computed changed. Pinned here on the shape that would most easily break
    // it if the count were "simplified" to populated cells.
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (old, new) = (wb_stray("x"), wb_stray("y"));
    let diff = compare_bytes(&old, &new).unwrap();
    assert_eq!(diff.metrics.cells_read, 2 * FAR_AREA);
}

#[test]
fn max_cells_read_fires_before_the_range_is_allocated() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (old, new) = (wb_stray("x"), wb_stray("y"));
    let opts = DiffOptions::builder()
        .limits(Limits {
            max_cells_read: Some(1_000),
            ..Limits::default()
        })
        .build()
        .unwrap();

    let (result, peak) = peak_growth(|| compare_bytes_with_options(&old, &new, opts));

    match result {
        Err(SheetsDiffError::LimitExceeded {
            limit: LimitKind::CellsRead,
            observed,
        }) => assert_eq!(
            observed, FAR_AREA,
            "the running bounding box at the cell that broke it"
        ),
        other => panic!("expected LimitExceeded{{CellsRead}}, got {other:?}"),
    }
    assert!(
        peak < BUDGET,
        "the bound fired, but only after {peak} bytes were spent: it must fire \
         before the allocation it exists to prevent"
    );
}

/// Cancelled from the `n`th poll onward, never before. Poll 1 is always the one
/// before each sheet pair, which any workbook reaches, so a test that cancelled
/// on it would pass without the reader ever polling.
struct CancelFromPoll(usize, AtomicUsize);

impl CancelFromPoll {
    fn new(n: usize) -> Self {
        Self(n, AtomicUsize::new(0))
    }
}

impl Cancellation for CancelFromPoll {
    fn is_cancelled(&self) -> bool {
        self.1.fetch_add(1, Ordering::SeqCst) + 1 >= self.0
    }
}

#[test]
fn cancellation_interrupts_a_read_before_its_range_exists() {
    // 60,000 populated cells (one poll interval is 50,000 records) and, on top,
    // a stray cell that makes the bounding box 200,001 x 200 = 40 million
    // positions. A dense read builds all of that before polling once.
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let build = |tag: &str| {
        let mut wb = Workbook::new();
        let ws = wb.add_worksheet();
        for r in 0..300u32 {
            for c in 0..200u16 {
                ws.write_string(r, c, format!("{tag}{r}_{c}")).unwrap();
            }
        }
        ws.write_string(FAR_ROW, 199, "stray").unwrap();
        wb.save_to_buffer().unwrap()
    };
    let (old, new) = (build("o"), build("n"));
    let opts = DiffOptions::builder()
        .cancellation(CancelFromPoll::new(2))
        .build()
        .unwrap();

    let (result, peak) = peak_growth(|| compare_bytes_with_options(&old, &new, opts));

    assert!(
        matches!(result, Err(SheetsDiffError::Cancelled)),
        "got {result:?}"
    );
    assert!(
        peak < BUDGET,
        "cancellation was observed, but only after {peak} bytes: it must be \
         observed within a poll interval of the stream, not after the range exists"
    );
}

#[test]
fn the_read_loop_polls_for_cancellation_on_its_own() {
    // The comparison phase polls too, at 50,000 coordinates, so a sheet of
    // 60,000 *populated* cells is cancelled by whichever poll comes first and
    // cannot show that the read loop polls. Styled blank cells can: each is a
    // record the reader must stream and count toward the poll interval, but
    // none is a populated cell, so the comparison has one coordinate and never
    // reaches its own poll. Only a poll inside the read loop can cancel this.
    //
    // The read is two streams over the sheet (values, then formulas), each
    // polling at its own crossing of the shared 50,000-record counter: 60,000
    // records put one poll in the value stream (at 50,000) and one in the
    // formula stream (at 100,000). Cancelling from poll 3 (poll 1 is the sheet
    // pair's) therefore needs *both* to exist; the value stream's poll alone is
    // what is under test, and removing it leaves a single read poll, which does
    // not reach 3.
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let bold = rust_xlsxwriter::Format::new().set_bold();
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    for r in 0..300u32 {
        for c in 0..200u16 {
            ws.write_blank(r, c, &bold).unwrap();
        }
    }
    ws.write_string(0, 0, "one populated cell").unwrap();
    let old = wb.save_to_buffer().unwrap();
    let new = wb_stray("y");
    let opts = DiffOptions::builder()
        .cancellation(CancelFromPoll::new(3))
        .build()
        .unwrap();

    let (result, _) = peak_growth(|| compare_bytes_with_options(&old, &new, opts));

    assert!(
        matches!(result, Err(SheetsDiffError::Cancelled)),
        "60,000 streamed records must reach a poll in each read stream, got {result:?}"
    );
}

#[test]
fn a_stray_far_formula_is_read_sparsely() {
    // `worksheet_formula` had the same dense shape as `worksheet_range`, over
    // the bounding box of the *formula* cells, so a far formula was the same
    // problem through the other door.
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let build = |result: &str| {
        let mut wb = Workbook::new();
        let ws = wb.add_worksheet();
        ws.write_string(0, 0, "a").unwrap();
        ws.write_formula(FAR_ROW, FAR_COL, Formula::new("=1+1").set_result(result))
            .unwrap();
        wb.save_to_buffer().unwrap()
    };
    let (old, new) = (build("2"), build("3"));

    let (diff, peak) = peak_growth(|| compare_bytes(&old, &new).unwrap());

    assert_eq!(diff.sheets[0].cell_diffs.len(), 1);
    assert!(
        peak < BUDGET,
        "peak heap growth {peak} bytes: the formula text must not be read into a \
         dense range either"
    );
}

#[test]
fn formula_text_is_still_attached_to_its_cell() {
    // The formula pass now attaches text to cells read by the value pass. A
    // formula whose text changed, with an unchanged cached value, is reported as
    // a formula change at the right address.
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let build = |f: &str| {
        let mut wb = Workbook::new();
        let ws = wb.add_worksheet();
        ws.write_string(0, 0, "a").unwrap();
        ws.write_formula(5, 3, Formula::new(f).set_result("2"))
            .unwrap();
        wb.save_to_buffer().unwrap()
    };
    let diff = compare_bytes(build("=1+1"), build("=2*1")).unwrap();
    assert_eq!(diff.summary.formulas_changed, 1);
    assert_eq!(diff.summary.values_changed, 0);
}
