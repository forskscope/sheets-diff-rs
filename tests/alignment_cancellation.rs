//! Row alignment is cancellable (alignment follow-ups, item 1).
//!
//! 2.5.1 made the *read* cancellable and left this phase: a cancel requested 100 ms into a 1.2 s
//! alignment was observed at 1,208 ms, because nothing in `align.rs` — and nothing in the loops
//! that turn its mapping into the set of cells to compare — looked at the token. Alignment is the
//! one phase that is quadratic in rows, so it is where cancellation latency lives.
//!
//! Two places poll, and **each has its own test**:
//!
//! * the LCS table fill (`lcs_match`), once per table row — the first quadratic phase, and the one
//!   the report that started this blamed;
//! * the three loops that build the compared-coordinate set (`build_sheet_diff`), once per row —
//!   each scans a whole cell map per row, so they are O(rows x cells). **Measured, this is the
//!   larger cost on every shape tried**, not the fill: 4,900 rows x 3 columns spends 0.25 s in the
//!   fill and 0.99 s building the set (1.28 s in all — the "1.2 s alignment" that was reported);
//!   x 12 columns, 0.24 s against 4.4 s; x 24 columns, 0.25 s against 29.8 s. A poll in the fill
//!   alone would have left most of a large alignment uncancellable.
//!
//! **What is asserted is a ratio, never a wall-clock figure**: the cancelled run's elapsed time as
//! a fraction of the same comparison run to completion in the same process. The margin is stated at
//! each assertion. The polls are counted, so a token that trips at the Nth poll trips *there*.

mod support;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use rust_xlsxwriter::Workbook;
use sheets_diff::options::AlignmentMode;
use sheets_diff::{Cancellation, DiffOptions, SheetsDiffError, compare_bytes_with_options};

/// Counts every poll; from poll number `trip_at` on, reports cancelled. `trip_at == usize::MAX`
/// never cancels, which makes it a poll counter for an uncancelled run.
#[derive(Clone)]
struct Token {
    trip_at: usize,
    polls: Arc<AtomicUsize>,
}

impl Token {
    fn new(trip_at: usize) -> Self {
        Token {
            trip_at,
            polls: Arc::new(AtomicUsize::new(0)),
        }
    }
    fn polls(&self) -> usize {
        self.polls.load(Ordering::SeqCst)
    }
}

impl Cancellation for Token {
    fn is_cancelled(&self) -> bool {
        self.polls.fetch_add(1, Ordering::SeqCst) + 1 >= self.trip_at
    }
}

fn opts(mode: AlignmentMode, token: &Token) -> DiffOptions {
    DiffOptions::builder()
        .alignment(mode)
        .cancellation(token.clone())
        .build()
        .unwrap()
}

fn row_key() -> AlignmentMode {
    AlignmentMode::RowKey { columns: vec![1] }
}

/// `rows` rows, a unique id in column A and `cols - 1` further columns of short text; one cell
/// differs in the new workbook so the comparison has something to report.
fn sheet(rows: u32, cols: u16, changed: bool) -> Vec<u8> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    for r in 0..rows {
        ws.write_string(r, 0, format!("ID{r:06}")).unwrap();
        for c in 1..cols {
            let mut v = format!("r{r}c{c}");
            if changed && r == 3 && c == 1 {
                v.push('!');
            }
            ws.write_string(r, c, v).unwrap();
        }
    }
    wb.save_to_buffer().unwrap()
}

/// Run to completion under a counting token that never fires: (elapsed seconds, total polls).
fn uncancelled(old: &[u8], new: &[u8], mode: AlignmentMode) -> (f64, usize) {
    let token = Token::new(usize::MAX);
    let start = Instant::now();
    compare_bytes_with_options(old, new, opts(mode, &token)).expect("an uncancelled run succeeds");
    (start.elapsed().as_secs_f64(), token.polls())
}

/// How many times a `Positional` comparison of the pair polls: the polls that happen before, and
/// regardless of, any alignment.
fn positional_polls(old: &[u8], new: &[u8]) -> usize {
    uncancelled(old, new, AlignmentMode::Positional).1
}

/// Run under a token that trips at poll `trip_at`: (result is `Cancelled`, elapsed seconds, polls).
fn cancelled_at(old: &[u8], new: &[u8], mode: AlignmentMode, trip_at: usize) -> (bool, f64, usize) {
    let token = Token::new(trip_at);
    let start = Instant::now();
    let r = compare_bytes_with_options(old, new, opts(mode, &token));
    let elapsed = start.elapsed().as_secs_f64();
    (
        matches!(r, Err(SheetsDiffError::Cancelled)),
        elapsed,
        token.polls(),
    )
}

// ---------------------------------------------------------------------------
// The LCS table fill: a tall, narrow sheet
// ---------------------------------------------------------------------------

/// 4,000 rows, one column: a 16-million-cell table. Cancelling 50 polls in must stop within the
/// first ~1% of the fill, so the cancelled run is little more than the two workbook reads. Asserted
/// as under **a third** of the full run: observed about 12%, and about 48% with the fill's poll
/// removed (the comparison then runs the whole fill before the next poll). The exact poll counts in
/// the next two tests are the deterministic guard; this is the time-shaped one.
#[test]
fn a_cancel_during_the_lcs_fill_is_observed_during_the_fill() {
    let (old, new) = (sheet(4_000, 1, false), sheet(4_000, 1, true));
    let (full, total_polls) = uncancelled(&old, &new, row_key());

    let (was_cancelled, elapsed, polls) = cancelled_at(&old, &new, row_key(), 50);
    eprintln!(
        "[fill] full {full:.4}s over {total_polls} polls; cancelled at poll 50 after {elapsed:.4}s ({:.1}% of full)",
        100.0 * elapsed / full
    );
    assert!(was_cancelled, "the comparison must return Err(Cancelled)");
    assert_eq!(polls, 50, "and stop at the poll that fired, not run on");
    assert!(
        elapsed < full / 3.0,
        "cancelled after {elapsed:.4}s against {full:.4}s for the whole comparison: \
         the cancel was not observed during the LCS fill"
    );
}

/// The other mode that runs an LCS.
#[test]
fn a_cancel_during_row_signature_alignment_is_observed_too() {
    let (old, new) = (sheet(4_000, 1, false), sheet(4_000, 1, true));
    let mode = || AlignmentMode::RowSignature {
        sample_columns: None,
    };
    let (full, _) = uncancelled(&old, &new, mode());
    let (was_cancelled, elapsed, polls) = cancelled_at(&old, &new, mode(), 50);
    eprintln!(
        "[signature] full {full:.4}s; cancelled at poll 50 after {elapsed:.4}s ({:.1}% of full)",
        100.0 * elapsed / full
    );
    assert!(was_cancelled);
    assert_eq!(polls, 50);
    assert!(elapsed < full / 3.0, "{elapsed:.4}s vs {full:.4}s");
}

// ---------------------------------------------------------------------------
// The coordinate-set loops: a wide sheet
// ---------------------------------------------------------------------------

/// 1,500 rows x 12 columns: the fill is 2.25 million cells; building the coordinate set scans an
/// 18,000-entry map once per row, 27 million steps, and is much the larger cost. The fill polls once per
/// row (1,500 polls) and the coordinate loop once per matched row (1,500 more), so tripping a few
/// polls after the fill's are used up lands **inside the coordinate loop**. With that loop's poll
/// removed the token would never trip at all — nothing polls again after it in a sheet this size —
/// and the comparison would return `Ok`, so the first assertion is the deterministic one; the
/// ratio is only a sanity bound, **under three quarters** of the full run (22-26% unloaded, 45% with
/// 40 busy loops on 32 cores — it is the loosest here because the deterministic assertions carry it).
#[test]
fn a_cancel_during_the_coordinate_set_build_is_observed_during_it() {
    let (old, new) = (sheet(1_500, 12, false), sheet(1_500, 12, true));
    let (full, total_polls) = uncancelled(&old, &new, row_key());
    // The poll structure, exactly: what a `Positional` run polls (the sheet-pair check before any
    // alignment), one per LCS row (1,500) and one per matched row in the coordinate loop (1,500).
    // Exact, so removing any one of those polls fails here whatever the timing does.
    let before_alignment = positional_polls(&old, &new);
    assert_eq!(total_polls, before_alignment + 1_500 + 1_500);
    // Trip 5 polls into the coordinate loop, after every fill poll is used up.
    let trip_at = before_alignment + 1_500 + 5;

    let (was_cancelled, elapsed, polls) = cancelled_at(&old, &new, row_key(), trip_at);
    eprintln!(
        "[coords] full {full:.4}s over {total_polls} polls; cancelled at poll {trip_at} after {elapsed:.4}s ({:.1}% of full)",
        100.0 * elapsed / full
    );
    assert!(
        was_cancelled,
        "no cancellation observed in the coordinate-set loops"
    );
    assert_eq!(polls, trip_at);
    assert!(
        elapsed < full * 0.75,
        "cancelled after {elapsed:.4}s against {full:.4}s for the whole comparison"
    );
}

/// Every row of the old sheet is removed and every row of the new one inserted (no id in common),
/// so the coordinate set is built by the *removed* and *inserted* loops, not the matched one. Each
/// polls once per row; the exact count pins that, and a cancel in each is observed there.
#[test]
fn the_removed_and_inserted_row_loops_poll_too() {
    let renamed = |changed: bool| {
        let mut wb = Workbook::new();
        let ws = wb.add_worksheet();
        for r in 0..1_000u32 {
            let id = if changed { "NEW" } else { "OLD" };
            ws.write_string(r, 0, format!("{id}{r:06}")).unwrap();
            for c in 1..8u16 {
                ws.write_string(r, c, format!("r{r}c{c}")).unwrap();
            }
        }
        wb.save_to_buffer().unwrap()
    };
    let (old, new) = (renamed(false), renamed(true));
    let before_alignment = positional_polls(&old, &new);
    let (_, total_polls) = uncancelled(&old, &new, row_key());
    // The fill (1,000 rows), then the removed loop (1,000 rows), then the inserted loop (1,000).
    assert_eq!(total_polls, before_alignment + 3_000);

    let in_removed = before_alignment + 1_000 + 5;
    let in_inserted = before_alignment + 2_000 + 5;
    for trip_at in [in_removed, in_inserted] {
        let (was_cancelled, _, polls) = cancelled_at(&old, &new, row_key(), trip_at);
        assert!(was_cancelled, "no cancellation observed at poll {trip_at}");
        assert_eq!(polls, trip_at);
    }
}

// ---------------------------------------------------------------------------
// Between the read and the alignment, and the uncancelled result
// ---------------------------------------------------------------------------

/// A token that fires at the first poll after the reads must cancel before any alignment work.
#[test]
fn a_cancel_between_the_read_and_the_alignment_is_observed() {
    let (old, new) = (sheet(4_000, 1, false), sheet(4_000, 1, true));
    let (full, _) = uncancelled(&old, &new, row_key());
    let (was_cancelled, elapsed, polls) = cancelled_at(&old, &new, row_key(), 2);
    eprintln!(
        "[between] full {full:.4}s; cancelled at poll 2 after {elapsed:.4}s ({:.1}% of full)",
        100.0 * elapsed / full
    );
    assert!(was_cancelled);
    assert_eq!(polls, 2);
    assert!(elapsed < full / 3.0);
}

/// Adding a poll must not change an answer: the same comparison with a token that never fires
/// gives a result equal, field for field, to the one with no token at all.
#[test]
fn an_uncancelled_comparison_is_unchanged_by_the_polls() {
    for mode in [
        AlignmentMode::Positional,
        row_key(),
        AlignmentMode::RowSignature {
            sample_columns: None,
        },
    ] {
        let (old, new) = (sheet(300, 4, false), sheet(300, 4, true));
        let plain = DiffOptions::builder()
            .alignment(mode.clone())
            .build()
            .unwrap();
        let without = compare_bytes_with_options(&old, &new, plain).unwrap();
        let token = Token::new(usize::MAX);
        let with = compare_bytes_with_options(&old, &new, opts(mode.clone(), &token)).unwrap();
        assert_eq!(with, without, "{mode:?}");
        assert!(matches!(mode, AlignmentMode::Positional) || token.polls() > 0);
    }
}
