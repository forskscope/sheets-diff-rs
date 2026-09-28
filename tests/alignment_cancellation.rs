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
//! * the three loops that build the compared-coordinate set (`build_sheet_diff`), once per row.
//!   **When this was written they were the larger cost on every shape tried** — each scanned a whole
//!   cell map per row, O(rows x cells): 4,900 rows x 3 columns spent 0.25 s in the fill and 0.99 s
//!   building the set (1.28 s in all — the "1.2 s alignment" that was reported), x 24 columns 0.25 s
//!   against 29.8 s. **f131 made them range queries and they are now cheap**, but they are still
//!   O(rows), still worth a poll, and the polls are still pinned here.
//!
//! **Every assertion in this file is a poll count. None is a time.** That is the second thing f131
//! settled, and it took a red CI to settle it. Four of these tests used to bound the cancelled run's
//! elapsed time as a fraction of the same comparison run to completion — a proxy for "the cancel was
//! observed *in* this phase". The proxy only holds while the phase dominates the run. It stopped
//! holding twice in one unit: f131 made the coordinate loops cheap, so the cancelled run became ~83%
//! of the full one; and the fill's own ratio, retuned onto a 8,000-row fixture, was then measuring
//! the **allocate-and-zero of a 256 MB table plus two workbook reads** — shared by both runs — with
//! the fill a sliver on top. On the Windows CI runners the cancelled run came out at 72-108% of the
//! full one and three jobs went red.
//!
//! What replaced it is exact and machine-independent: an aligned run of a fully-matching pair polls
//! `positional_polls + rows` (the fill, once per table row) `+ rows` (the coordinate loop, once per
//! matched row). Remove any one poll and a count is short by exactly the row count — **removing the
//! fill's poll fails four of these tests**, where the ratio failed one, and it fails them at any
//! speed. The elapsed times are still printed, because they are useful to read; nothing asserts on
//! them. The polls are counted, so a token that trips at the Nth poll trips *there*.

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

/// `max_alignment_product` is switched off so the tall fixtures below (8,000 rows: a 64-million-cell
/// table, over the 25-million default) really align instead of falling back to `Positional`.
fn opts(mode: AlignmentMode, token: &Token) -> DiffOptions {
    DiffOptions::builder()
        .max_alignment_product(None)
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

/// 4,000 rows, one column. **The fill's poll is pinned by an exact count, not by a clock:** every row
/// has an id and every id matches, so an aligned run polls once before alignment, once per LCS table
/// row, and once per matched row in the coordinate loop — `before_alignment + rows + rows`, exactly.
/// Delete the fill's poll and this count is short by `rows`, on any machine, at any speed.
///
/// **There was a ratio assertion here and it had to go** (see the module comment): the cancelled run
/// pays the same two workbook reads and the same allocate-and-zero of a `rows²` `u32` table as the
/// full run, and only the *fill* on top is what the cancel cuts short. On a fast machine the fill
/// dominates and the ratio looked like a measurement; on the Windows CI runners at 8,000 rows
/// (a 256 MB table) the shared cost swamped it and the cancelled run came out at 72-108% of the full
/// one. It was reading allocation and calling it the fill, so the count replaces it.
#[test]
fn a_cancel_during_the_lcs_fill_is_observed_during_the_fill() {
    let (old, new) = (sheet(4_000, 1, false), sheet(4_000, 1, true));
    let (full, total_polls) = uncancelled(&old, &new, row_key());
    let before_alignment = positional_polls(&old, &new);
    assert_eq!(
        total_polls,
        before_alignment + 4_000 + 4_000,
        "one poll per LCS table row and one per matched row in the coordinate loop"
    );

    let (was_cancelled, elapsed, polls) = cancelled_at(&old, &new, row_key(), 50);
    eprintln!(
        "[fill] full {full:.4}s over {total_polls} polls; cancelled at poll 50 after {elapsed:.4}s ({:.1}% of full)",
        100.0 * elapsed / full
    );
    assert!(was_cancelled, "the comparison must return Err(Cancelled)");
    assert_eq!(polls, 50, "and stop at the poll that fired, not run on");
}

/// The other mode that runs an LCS. Same exact-count guard, same reason.
#[test]
fn a_cancel_during_row_signature_alignment_is_observed_too() {
    let (old, new) = (sheet(4_000, 1, false), sheet(4_000, 1, true));
    let mode = || AlignmentMode::RowSignature {
        sample_columns: None,
    };
    let (full, total_polls) = uncancelled(&old, &new, mode());
    let before_alignment = positional_polls(&old, &new);
    assert_eq!(total_polls, before_alignment + 4_000 + 4_000);
    let (was_cancelled, elapsed, polls) = cancelled_at(&old, &new, mode(), 50);
    eprintln!(
        "[signature] full {full:.4}s; cancelled at poll 50 after {elapsed:.4}s ({:.1}% of full)",
        100.0 * elapsed / full
    );
    assert!(was_cancelled);
    assert_eq!(polls, 50);
}

// ---------------------------------------------------------------------------
// The coordinate-set loops: a wide sheet
// ---------------------------------------------------------------------------

/// 1,500 rows x 12 columns. The fill polls once per row (1,500 polls) and the coordinate loop once
/// per matched row (1,500 more), so tripping a few polls after the fill's are used up lands **inside
/// the coordinate loop**. With that loop's poll removed the token would never trip at all — nothing
/// polls again after it in a sheet this size — and the comparison would return `Ok`, and the exact
/// count of polls would be off by 1,500: both deterministic. **There is no ratio assertion here any
/// more.** It was "under three quarters of the full run", true while the loop was the expensive
/// part; f131 made the loop cheap, the cancelled run is then most of the full one, and a timing
/// bound would be asserting the wrong thing.
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

/// A token that fires at the **first poll alignment makes** must stop there. `positional_polls` is
/// what this pair polls before alignment begins (1: the sheet-pair check), so the poll index is
/// derived rather than assumed, and the assertion is an equality on where the run stopped. (That the
/// poll it stops at belongs to the *fill* is the previous tests' business, by count.)
#[test]
fn a_cancel_between_the_read_and_the_alignment_is_observed() {
    let (old, new) = (sheet(4_000, 1, false), sheet(4_000, 1, true));
    let trip_at = positional_polls(&old, &new) + 1;
    let (full, _) = uncancelled(&old, &new, row_key());
    let (was_cancelled, elapsed, polls) = cancelled_at(&old, &new, row_key(), trip_at);
    eprintln!(
        "[between] full {full:.4}s; cancelled at poll {trip_at} after {elapsed:.4}s ({:.1}% of full)",
        100.0 * elapsed / full
    );
    assert!(was_cancelled);
    assert_eq!(polls, trip_at);
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
