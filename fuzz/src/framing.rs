// Shared by `fuzz_open_xlsx_bytes.rs`, its permanent guard (`tests/`, main crate) and the corpus
// generator (`examples/gen-fuzz-corpus.rs`, main crate), via `include!` — a plain-text splice, not
// a crate dependency, so `fuzz/` still depends on nothing extra and the main crate does not gain
// `fuzz/` as a dependency (M9 unit 01, replacing the midpoint split). Pure `std`, no crate
// references, so it compiles unchanged in all three places.
//
// **The defect this replaces.** The target used to split its input at `data.len() / 2`, so a seed
// had to be two valid workbooks concatenated **and of exactly equal length** or the split landed
// inside one of them and both halves became invalid. A length prefix makes the split explicit: a
// seed built by [`make_seed`] round-trips through [`split_old_new`] to exactly the two byte
// strings it was built from, whatever their lengths.
//
// **Format:** a 4-byte little-endian length prefix for `old`, then `old`'s bytes, then everything
// else as `new`.

/// Split `data` into `(old, new)` per the framing above.
///
/// If the prefix does not fit in `data`, or claims more bytes than remain, `old` is empty and `new`
/// is everything that follows the (possibly absent) prefix — an input that cannot decode to two
/// workbooks, which reaches [`sheets_diff::compare_bytes`] as a pair the reader will reject, the
/// same "not two valid inputs" outcome an out-of-range prefix already produces. Never panics: every
/// arithmetic step is a bounds check first.
#[allow(dead_code)] // one of the two functions below is unused in whichever file does not need it
pub fn split_old_new(data: &[u8]) -> (&[u8], &[u8]) {
    if data.len() < 4 {
        return (&[], data);
    }
    let len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    let rest = &data[4..];
    if len > rest.len() {
        return (&[], rest);
    }
    rest.split_at(len)
}

/// Build a seed file from two byte strings — the exact inverse of [`split_old_new`]: for any `old`
/// and `new`, `split_old_new(&make_seed(old, new)) == (old, new)`.
#[allow(dead_code)]
pub fn make_seed(old: &[u8], new: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(4 + old.len() + new.len());
    v.extend_from_slice(&(old.len() as u32).to_le_bytes());
    v.extend_from_slice(old);
    v.extend_from_slice(new);
    v
}
