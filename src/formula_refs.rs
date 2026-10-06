//! Row-reference mapping for formula text (formulas-move-with-their-rows, unit 01).
//!
//! Excel rewrites the row numbers inside a formula when rows move, so a formula compared as raw text across a
//! row alignment looks changed when it is not. [`map_row_references`] takes the **old** cell's formula and the
//! alignment's old-to-new row mapping, and returns the text with every row reference mapped, or **`None`**.
//!
//! **`None` is the contract, not an error.** It means some part of the formula was not confidently understood,
//! so no mapped text is offered and the caller must behave as though this function did not exist. A wrong mapped
//! text would become a wrong annotation, and a wrong annotation says a formula did not change when it did. So
//! this function is judged by what it declines, and the default for anything unrecognised is to decline the
//! **whole formula**, never just the token.
//!
//! How, in one pass over the characters, never by searching the text for substrings:
//!
//! * Whitespace passes through. The **boundary set** is `+ - * / ^ & = < > ( ) , : %`: each is a token of its own
//!   and ends any word. Every other character refuses the formula.
//! * A token starting with a digit or `.` is read as a **number** first, and must be followed by a boundary. Numbers
//!   are taken before identifiers, which is why `LOG10` (it starts with a letter) is one word and cannot be read as
//!   column `LOG`, row 10.
//! * A **word** is a maximal run of `A-Z a-z 0-9 _ . $` starting with a letter, `_` or `$`.
//!   - A defined name (case-insensitive, supplied by the caller from the file) refuses, whatever its shape. A word
//!     shaped like a cell reference is a reference in every file Excel wrote, because Excel forbids such names, but a
//!     file this crate did not produce is under no obligation to follow that, so the names come from the file.
//!     `calamine` reports no scope, so a name scoped to another sheet refuses a formula on this one. That
//!     over-refusal is deliberate and safe: do not "fix" it.
//!   - A word shaped like an A1 reference (`$?` 1-3 letters `$?` digits) followed by `(` refuses: `LOG10(`,
//!     `ATAN2(`, `DEC2BIN(` are function names that read as references.
//!   - A word shaped like an A1 reference but out of range or written with a leading zero (`ZZZ1`, `A05`) refuses.
//!     The mapped text could not byte-match, because the zero would be lost.
//!   - Any other word followed by `(` is a **function name** and passes through. We hold no function list and want
//!     none: a function name is never rewritten and its arguments are tokenized like any others.
//!   - `TRUE` and `FALSE` pass through. Everything else refuses (defined names, `C` in `C:C`, `R5C3`, ...).
//! * A `:` is accepted only between two A1 references (so whole-column and whole-row forms refuse), and a range
//!   written backwards, or mapped backwards, refuses. **Ranges are mapped only on a sheet whose row mapping is
//!   monotone** (see [`RowMap`]); on any other sheet every range refuses and single references are unaffected.
//! * A reference whose row is not in `matched` refuses (Excel makes those `#REF!`).
//!
//! Rows are mapped through `matched`, never by offset arithmetic: a reference above an edit does not move and one
//! below it does, and only the mapping knows which. Everything but the row digits of a reference is preserved byte
//! for byte, including `$`, case and whitespace, because the result is compared for equality against the new text.
//! `$` does not stop a reference moving on insertion; it governs copy and fill.

use std::collections::{BTreeMap, BTreeSet};

/// The highest column (`XFD`) and row Excel has.
const MAX_COLUMN: u32 = 16_384;
const MAX_ROW: u64 = 1_048_576;

/// Why a formula was refused. Internal: the public contract is `Option`. Kept distinct so the reasons can be
/// counted over real formulas, to show what refusing costs.
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub(crate) enum Refusal {
    NotAscii,
    UnexpectedCharacter,
    BadNumber,
    DefinedName,
    A1ShapedFunction,
    A1ShapedOutOfRange,
    UnrecognisedWord,
    MisplacedColon,
    BackwardsRange,
    NonMonotoneMapping,
    RowNotMatched,
}

/// A sheet pair's row mapping, with the one property ranges depend on computed once.
///
/// **The row mapping is injective, by the row-alignment invariant, and is not necessarily monotone; the
/// `monotone` check is why.** Injectivity is clause 4 and it holds. Order is a different property and nothing
/// asserts it: under `RowKey` the keyless rescue pairs identical rows among the unpaired ones in their own row
/// order, with no knowledge of where the LCS pairs sit, so the union can interleave
/// (`{1->1, 2->3, 3->2}` for old `K1/spacer/K2`, new `K1/K2/spacer`). Mapping a range by its two endpoints is exact
/// if and only if the mapping is monotone: then every interior row lands inside the mapped endpoints and, by
/// injectivity, no outside row does. On a non-monotone sheet `C1:C2` would map to `C1:C3`, a range that contains
/// the image of old row 3, which the original did not. Do not remove this check as redundant.
pub(crate) struct RowMap<'a> {
    matched: &'a BTreeMap<u32, u32>,
    monotone: bool,
}

impl<'a> RowMap<'a> {
    /// One pass over `matched`. Build this once per sheet pair, not once per formula.
    pub(crate) fn new(matched: &'a BTreeMap<u32, u32>) -> Self {
        let monotone = matched
            .values()
            .zip(matched.values().skip(1))
            .all(|(a, b)| a < b);
        RowMap { matched, monotone }
    }
}

/// Normalises names the way `meta.rs` does when it matches defined names (`to_lowercase`), so a caller builds the
/// set with the same rule this module applies to each word.
pub(crate) fn normalise_defined_names<'a>(
    names: impl IntoIterator<Item = &'a str>,
) -> BTreeSet<String> {
    names.into_iter().map(str::to_lowercase).collect()
}

/// Maps every row reference in `raw` (an **old** cell's formula) through `matched` (old row to new row), or
/// returns `None` if any part of the formula is not understood. See the module documentation.
///
/// `defined_names` must hold every defined name in the workbook, from [`normalise_defined_names`]. An empty set
/// means the workbook has none, which is a fact and not a fallback.
pub(crate) fn map_row_references(
    raw: &str,
    matched: &RowMap<'_>,
    defined_names: &BTreeSet<String>,
) -> Option<String> {
    analyse(raw, matched, defined_names).ok()
}

enum Tok<'a> {
    Ws(&'a str),
    Op(char),
    Number(&'a str),
    /// A function name or boolean, passed through.
    Pass(&'a str),
    Ref(Ref<'a>),
}

/// An A1 reference: the text before the row digits, and the row.
struct Ref<'a> {
    before_row: &'a str,
    row: u32,
}

pub(crate) fn analyse(
    raw: &str,
    matched: &RowMap<'_>,
    defined_names: &BTreeSet<String>,
) -> Result<String, Refusal> {
    if !raw.is_ascii() {
        return Err(Refusal::NotAscii);
    }
    let toks = tokenize(raw, defined_names)?;
    check_colons(&toks, matched)?;
    let mut out = String::with_capacity(raw.len());
    for tok in &toks {
        match tok {
            Tok::Ws(s) | Tok::Number(s) | Tok::Pass(s) => out.push_str(s),
            Tok::Op(c) => out.push(*c),
            Tok::Ref(r) => {
                let new_row = matched.matched.get(&r.row).ok_or(Refusal::RowNotMatched)?;
                out.push_str(r.before_row);
                out.push_str(&new_row.to_string());
            }
        }
    }
    Ok(out)
}

fn tokenize<'a>(raw: &'a str, defined_names: &BTreeSet<String>) -> Result<Vec<Tok<'a>>, Refusal> {
    let b = raw.as_bytes();
    let mut raw_toks: Vec<(usize, usize, Kind)> = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        let start = i;
        if matches!(c, b' ' | b'\t' | b'\r' | b'\n') {
            while i < b.len() && matches!(b[i], b' ' | b'\t' | b'\r' | b'\n') {
                i += 1;
            }
            raw_toks.push((start, i, Kind::Ws));
        } else if b"+-*/^&=<>(),:%".contains(&c) {
            i += 1;
            raw_toks.push((start, i, Kind::Op));
        } else if c.is_ascii_digit() || c == b'.' {
            i = read_number(b, i)?;
            raw_toks.push((start, i, Kind::Number));
        } else if c.is_ascii_alphabetic() || c == b'_' || c == b'$' {
            while i < b.len()
                && (b[i].is_ascii_alphanumeric() || matches!(b[i], b'_' | b'.' | b'$'))
            {
                i += 1;
            }
            raw_toks.push((start, i, Kind::Word));
        } else {
            return Err(Refusal::UnexpectedCharacter);
        }
    }

    let mut toks = Vec::with_capacity(raw_toks.len());
    for (n, &(s, e, kind)) in raw_toks.iter().enumerate() {
        let text = &raw[s..e];
        toks.push(match kind {
            Kind::Ws => Tok::Ws(text),
            Kind::Op => Tok::Op(char::from(b[s])),
            Kind::Number => Tok::Number(text),
            Kind::Word => {
                let followed_by_paren = raw_toks[n + 1..]
                    .iter()
                    .find(|t| !matches!(t.2, Kind::Ws))
                    .is_some_and(|t| t.2 == Kind::Op && b[t.0] == b'(');
                classify_word(text, followed_by_paren, defined_names)?
            }
        });
    }
    Ok(toks)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Ws,
    Op,
    Number,
    Word,
}

/// Reads a number starting at `i`: `digits [. digits] [E[+-]digits]`, or `. digits`. It must end at a boundary.
fn read_number(b: &[u8], mut i: usize) -> Result<usize, Refusal> {
    let mut digits = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
        digits += 1;
    }
    if i < b.len() && b[i] == b'.' {
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return Err(Refusal::BadNumber);
    }
    if i < b.len() && matches!(b[i], b'e' | b'E') {
        let mut j = i + 1;
        if j < b.len() && matches!(b[j], b'+' | b'-') {
            j += 1;
        }
        let exp_start = j;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j > exp_start {
            i = j;
        }
    }
    if i < b.len() && (b[i].is_ascii_alphanumeric() || matches!(b[i], b'_' | b'.' | b'$')) {
        return Err(Refusal::BadNumber);
    }
    Ok(i)
}

fn classify_word<'a>(
    word: &'a str,
    followed_by_paren: bool,
    defined_names: &BTreeSet<String>,
) -> Result<Tok<'a>, Refusal> {
    if defined_names.contains(&word.to_lowercase()) {
        return Err(Refusal::DefinedName);
    }
    if let Some(shape) = a1_shape(word) {
        if followed_by_paren {
            return Err(Refusal::A1ShapedFunction);
        }
        return shape
            .into_ref(word)
            .map(Tok::Ref)
            .ok_or(Refusal::A1ShapedOutOfRange);
    }
    if word.contains('$') {
        return Err(Refusal::UnrecognisedWord);
    }
    if followed_by_paren || word.eq_ignore_ascii_case("true") || word.eq_ignore_ascii_case("false")
    {
        return Ok(Tok::Pass(word));
    }
    Err(Refusal::UnrecognisedWord)
}

/// The structure of a word shaped like an A1 reference, before its range is checked.
struct A1Shape<'a> {
    column: &'a str,
    /// Index where the row digits begin.
    row_start: usize,
    row_digits: &'a str,
}

impl<'a> A1Shape<'a> {
    /// `Some` only if the column is within `XFD`, the row is within range and has no leading zero.
    fn into_ref(self, word: &'a str) -> Option<Ref<'a>> {
        let col = self.column.bytes().fold(0u32, |acc, c| {
            acc * 26 + u32::from(c.to_ascii_uppercase() - b'A' + 1)
        });
        if col > MAX_COLUMN || self.row_digits.starts_with('0') {
            return None;
        }
        let row: u64 = self.row_digits.parse().ok()?;
        if !(1..=MAX_ROW).contains(&row) {
            return None;
        }
        Some(Ref {
            before_row: &word[..self.row_start],
            row: u32::try_from(row).ok()?,
        })
    }
}

/// `$? letters{1,3} $? digits+`, and nothing else.
fn a1_shape(word: &str) -> Option<A1Shape<'_>> {
    let b = word.as_bytes();
    let mut i = usize::from(b.first() == Some(&b'$'));
    let col_start = i;
    while i < b.len() && b[i].is_ascii_alphabetic() {
        i += 1;
    }
    let col_len = i - col_start;
    if !(1..=3).contains(&col_len) {
        return None;
    }
    let column = &word[col_start..i];
    if b.get(i) == Some(&b'$') {
        i += 1;
    }
    let row_start = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if i == row_start || i != b.len() {
        return None;
    }
    Some(A1Shape {
        column,
        row_start,
        row_digits: &word[row_start..],
    })
}

fn first_significant<'a, 'b>(mut toks: impl Iterator<Item = &'b Tok<'a>>) -> Option<&'b Tok<'a>>
where
    'a: 'b,
{
    toks.find(|t| !matches!(t, Tok::Ws(_)))
}

/// A `:` is valid only between two A1 references, and the mapped range must not run backwards.
fn check_colons(toks: &[Tok<'_>], matched: &RowMap<'_>) -> Result<(), Refusal> {
    for (n, tok) in toks.iter().enumerate() {
        if !matches!(tok, Tok::Op(':')) {
            continue;
        }
        let left = first_significant(toks[..n].iter().rev());
        let right = first_significant(toks[n + 1..].iter());
        let (Some(Tok::Ref(l)), Some(Tok::Ref(r))) = (left, right) else {
            return Err(Refusal::MisplacedColon);
        };
        if !matched.monotone {
            return Err(Refusal::NonMonotoneMapping);
        }
        // An unmatched row is refused when the references are rebuilt; only order is checked here.
        if let (Some(ml), Some(mr)) = (matched.matched.get(&l.row), matched.matched.get(&r.row))
            && (l.row > r.row || ml > mr)
        {
            return Err(Refusal::BackwardsRange);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
