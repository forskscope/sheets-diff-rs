// Shared by `fuzz_self_comparison.rs`, its permanent guard (`tests/`, main crate) and the corpus
// generator (`examples/gen-fuzz-corpus.rs`, main crate), via `include!` (see `framing.rs` for why
// that mechanism and not a crate dependency). Uses `sheets_diff`'s public API only, which every one
// of the three including files already depends on (the fuzz crate as an external dependency named
// `sheets_diff`; the main crate's own tests and examples as themselves).
//
// **Format:** a 3-byte header, then the rest of the input is the workbook — one `.xlsx` candidate,
// compared with itself. No framing problem: nothing to split, so no invalid-by-construction case
// exists the way the paired target's midpoint split had.
//
// **Bounding what the header can select (Known risk 3).** Every value the header can choose is a
// [`sheets_diff::Limits::hardened`] value or smaller — a fuzzer-chosen combination can never
// allocate more than `hardened()` already allows on its own (about 100 MB for the alignment table
// at its default bound, `DEFAULT_MAX_ALIGNMENT_PRODUCT` — see `docs/src/maintainers/performance.md`
// — well under cargo-fuzz's default `-rss_limit_mb=2048`). `max_cells_read` is the one field the
// header can additionally *shrink*; it is never grown past `hardened()`'s own value.

/// The fixed header length. Exported so the target, the guard and the generator agree on where the
/// header ends without repeating the literal.
#[allow(dead_code)]
pub const SELF_SEED_HEADER_LEN: usize = 3;

/// Build the options a header byte string selects. `header.len()` must be
/// [`SELF_SEED_HEADER_LEN`]; shorter is treated as zero-filled, which is a valid header (selects
/// `Positional`, `hardened()` unmodified) rather than a panic.
#[allow(dead_code)]
pub fn opts_from_header(header: &[u8]) -> sheets_diff::DiffOptions {
    let b = |i: usize| header.get(i).copied().unwrap_or(0);
    let mode = match b(0) % 3 {
        0 => sheets_diff::AlignmentMode::Positional,
        1 => sheets_diff::AlignmentMode::RowKey {
            // 1-based (src/options.rs), 1..=5: enough to land on a real column in every generated
            // seed without ever driving the row-key extraction over an unbounded column count.
            columns: vec![1 + (b(1) % 5) as u32],
        },
        _ => sheets_diff::AlignmentMode::RowSignature {
            sample_columns: None,
        },
    };
    let mut limits = sheets_diff::Limits::hardened();
    // Shrink, never grow: b(2) == 0 keeps hardened()'s own 5,000,000; any other byte picks a smaller
    // bound, so every input this target can construct stays inside hardened()'s own ceiling.
    if b(2) != 0 {
        limits.max_cells_read = Some((b(2) as u64 + 1) * 1_000);
    }
    sheets_diff::DiffOptions::builder()
        .alignment(mode)
        .limits(limits)
        .build()
        .expect("no combination this header can construct is invalid (RFC-037 §3.3)")
}

/// Split a self-comparison seed into its header and workbook bytes.
#[allow(dead_code)]
pub fn split_self_seed(data: &[u8]) -> (&[u8], &[u8]) {
    if data.len() < SELF_SEED_HEADER_LEN {
        return (data, &[]);
    }
    data.split_at(SELF_SEED_HEADER_LEN)
}

/// Build a seed file from a header and workbook bytes — the inverse of [`split_self_seed`].
#[allow(dead_code)]
pub fn make_self_seed(header: [u8; SELF_SEED_HEADER_LEN], workbook: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(SELF_SEED_HEADER_LEN + workbook.len());
    v.extend_from_slice(&header);
    v.extend_from_slice(workbook);
    v
}

/// The self-comparison oracle: `diff` is the result of comparing a workbook with itself. Panics
/// (via the assertions) describing which sheet and what failed, so a violation is legible directly
/// from `cargo fuzz run`'s output or a failing test, without a debugger.
#[allow(dead_code)]
pub fn assert_self_comparison_is_empty(diff: &sheets_diff::WorkbookDiff) {
    assert_eq!(
        diff.summary.cells_changed, 0,
        "a workbook compared with itself reported {} changed cell(s)",
        diff.summary.cells_changed
    );
    for s in &diff.sheets {
        assert!(
            s.cell_diffs.is_empty(),
            "sheet {:?}: {} cell diff(s) against itself",
            s.new_sheet.as_ref().or(s.old_sheet.as_ref()).map(|r| &r.name),
            s.cell_diffs.len()
        );
        assert!(
            matches!(s.change, sheets_diff::SheetChange::Unchanged),
            "sheet {:?}: change was {:?}, not Unchanged",
            s.new_sheet.as_ref().or(s.old_sheet.as_ref()).map(|r| &r.name),
            s.change
        );
    }
}
