//! The permanent guard (M9 unit 01): the fuzz corpus must keep reaching the sheet reader.
//!
//! Through this unit, every seed in `fuzz/corpus/fuzz_open_xlsx_bytes/` failed at
//! `not an xlsx file` — the target's own midpoint-split framing made a valid seed (two workbooks,
//! concatenated, of exactly equal length) nearly impossible to write, so nobody had. A coverage
//! number taken once cannot catch this coming back; this test runs in the ordinary gate, on every
//! push, and fails the day the corpus is minimised back down to the empty file, or the framing
//! regresses, or a fix to the reader stops being exercised by what is here.
//!
//! **One framing function per target, shared with the target and the corpus generator via
//! `include!`** (`fuzz/src/framing.rs`, `fuzz/src/self_seed.rs`) — not a second copy that can drift
//! from what the fuzzer actually runs. `include!` splices the file's text in directly; it does not
//! add `fuzz/` as a dependency of this crate (rule 003's boundary stays: the only compile check on
//! `fuzz/` remains `cargo check --manifest-path fuzz/Cargo.toml --bins`).
//!
//! Two corpora, two things asserted per criterion 2:
//! - `fuzz_open_xlsx_bytes`: at least one seed decodes, under the target's own framing, to two
//!   inputs `compare_bytes` opens successfully — i.e. reaches past the archive into the reader.
//! - `fuzz_self_comparison`: at least one seed reaches the reader (`compare_bytes_with_options`
//!   returns `Ok`), and — the oracle, criterion 3 — **every** seed that does yields zero cell diffs,
//!   in whichever alignment mode its header selects. `sheets_diff::compare_bytes*` never panics on
//!   malformed input (RFC-005); if it did, this test would abort like any other, which is already a
//!   failure and needs no assertion to say so.

include!("../fuzz/src/framing.rs");
include!("../fuzz/src/self_seed.rs");

/// Every committed seed for `target`, from the live corpus **and** from `fuzz/corpus-quarantine/`.
///
/// A quarantined seed is one libFuzzer must not *mutate* in CI because a mutation of it reaches a
/// crash we have not fixed (see that directory's README). Reading it once, as itself, is safe and is
/// exactly what these tests do — so quarantining a seed takes it out of the fuzzer's reach without
/// taking it out of the guard's. Without this, a quarantined seed would rot: still committed, no
/// longer decoded by anything.
fn corpus_files(target: &str) -> Vec<std::path::PathBuf> {
    let live = std::path::Path::new("fuzz/corpus").join(target);
    let quarantine = std::path::Path::new("fuzz/corpus-quarantine").join(target);

    // The live corpus must exist; the quarantine directory is absent whenever nothing is
    // quarantined, which is the state we want to be in.
    let mut files = read_seeds(&live).unwrap_or_else(|e| panic!("{}: {e}", live.display()));
    match read_seeds(&quarantine) {
        Ok(more) => files.extend(more),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => panic!("{}: {e}", quarantine.display()),
    }

    files.sort();
    assert!(!files.is_empty(), "{}: no seeds committed", live.display());
    files
}

fn read_seeds(dir: &std::path::Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut v = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let p = entry?.path();
        if p.is_file() {
            v.push(p);
        }
    }
    Ok(v)
}

#[test]
fn the_open_xlsx_bytes_corpus_has_a_seed_that_reaches_the_reader() {
    let mut reached = Vec::new();
    for path in corpus_files("fuzz_open_xlsx_bytes") {
        let data = std::fs::read(&path).unwrap();
        let (old, new) = split_old_new(&data);
        if let Ok(diff) = sheets_diff::compare_bytes(old, new) {
            assert!(
                !diff.sheets.is_empty() || !old.is_empty() || !new.is_empty(),
                "{}: reached Ok with no sheets and empty input — not a real case",
                path.display()
            );
            reached.push(path.file_name().unwrap().to_string_lossy().into_owned());
        }
    }
    assert!(
        !reached.is_empty(),
        "no seed in fuzz/corpus/fuzz_open_xlsx_bytes/ reaches past the archive into the sheet \
         reader — every one fails at open(); this is exactly the defect M9 unit 01 fixed"
    );
    eprintln!("reached the reader: {reached:?}");
}

#[test]
fn the_self_comparison_corpus_has_a_seed_that_reaches_the_reader_and_every_seed_that_does_is_empty()
{
    let mut reached = Vec::new();
    for path in corpus_files("fuzz_self_comparison") {
        let data = std::fs::read(&path).unwrap();
        let (header, workbook) = split_self_seed(&data);
        let opts = opts_from_header(header);
        // An `Err` here just means this seed did not reach the oracle; not a failure by itself.
        if let Ok(diff) = sheets_diff::compare_bytes_with_options(workbook, workbook, opts) {
            assert_self_comparison_is_empty(&diff);
            reached.push(path.file_name().unwrap().to_string_lossy().into_owned());
        }
    }
    assert!(
        !reached.is_empty(),
        "no seed in fuzz/corpus/fuzz_self_comparison/ reaches the reader — the self-comparison \
         oracle was never exercised"
    );
    eprintln!("reached the reader, zero cell diffs: {reached:?}");
}

/// Guards the sharing itself: a header this crate's own `AlignmentMode::RowKey` accepts must be
/// interpreted as `RowKey` by `opts_from_header`, not silently fall through to `Positional` because
/// the shared file and the target disagree on the byte layout. Cheap and specific, unlike the two
/// tests above, which would not distinguish "always Positional" from "the header is read correctly"
/// on a workbook that happens to align the same way under every mode.
#[test]
fn opts_from_header_selects_the_mode_the_header_names() {
    assert!(matches!(
        opts_from_header(&[0, 0, 0]).matching.alignment,
        sheets_diff::AlignmentMode::Positional
    ));
    assert!(matches!(
        opts_from_header(&[1, 0, 0]).matching.alignment,
        sheets_diff::AlignmentMode::RowKey { .. }
    ));
    assert!(matches!(
        opts_from_header(&[2, 0, 0]).matching.alignment,
        sheets_diff::AlignmentMode::RowSignature { .. }
    ));
    if let sheets_diff::AlignmentMode::RowKey { columns } =
        opts_from_header(&[1, 2, 0]).matching.alignment
    {
        assert_eq!(columns, vec![3]); // 1 + (2 % 5)
    } else {
        panic!("header [1, 2, 0] must select RowKey");
    }
}
