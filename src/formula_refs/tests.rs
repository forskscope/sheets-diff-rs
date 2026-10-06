use super::*;

fn mr(raw: &str, m: &BTreeMap<u32, u32>, names: &BTreeSet<String>) -> Option<String> {
    map_row_references(raw, &RowMap::new(m), names)
}

fn an(raw: &str, m: &BTreeMap<u32, u32>, names: &BTreeSet<String>) -> Result<String, Refusal> {
    analyse(raw, &RowMap::new(m), names)
}

/// Rows below 5 stay put; rows 5 and up move down by one, as after inserting a row at 5. Old rows 1..=300.
fn insert_at_5() -> BTreeMap<u32, u32> {
    (1..=300)
        .map(|r| (r, if r >= 5 { r + 1 } else { r }))
        .collect()
}

fn no_names() -> BTreeSet<String> {
    BTreeSet::new()
}

fn map(raw: &str) -> Option<String> {
    mr(raw, &insert_at_5(), &no_names())
}

fn why(raw: &str) -> Refusal {
    an(raw, &insert_at_5(), &no_names()).unwrap_err()
}

// ---------------------------------------------------------------------------
// Refusals first. Each must return `None`.
// ---------------------------------------------------------------------------

/// Test 1. The function-name hazard. If this maps, the unit has failed.
#[test]
fn log10_is_a_function_name_not_column_log_row_ten() {
    assert_eq!(map("=LOG10(C5)"), None);
    assert_eq!(why("=LOG10(C5)"), Refusal::A1ShapedFunction);
}

/// Test 2. A reference-shaped string literal.
#[test]
fn a_string_literal_refuses_the_whole_formula() {
    assert_eq!(map(r#"=IF(A1="B2",1,2)"#), None);
    assert_eq!(why(r#"=IF(A1="B2",1,2)"#), Refusal::UnexpectedCharacter);
}

/// Test 3. Stage 2: other sheets.
#[test]
fn cross_sheet_references_refuse() {
    assert_eq!(map("=Sheet2!C5"), None);
    assert_eq!(map("='My Sheet'!C5"), None);
}

/// Test 4. Stage 2: whole-column and whole-row forms.
#[test]
fn whole_column_and_whole_row_forms_refuse() {
    assert_eq!(map("=SUM(C:C)"), None);
    assert_eq!(map("=SUM(5:5)"), None);
    assert_eq!(map("=SUM(C5:C)"), None);
    assert_eq!(why("=SUM(C:C)"), Refusal::UnrecognisedWord);
}

/// Test 5. A structured reference.
#[test]
fn structured_references_refuse() {
    assert_eq!(map("=Table1[Amount]"), None);
}

/// Test 6. A defined name.
#[test]
fn a_defined_name_refuses() {
    assert_eq!(map("=MyName*2"), None);
}

/// Test 7. A reference to a row absent from `matched`.
#[test]
fn a_reference_to_an_unpaired_row_refuses() {
    let mut m = insert_at_5();
    m.remove(&7);
    assert_eq!(mr("=C7*2", &m, &no_names()), None);
    assert_eq!(an("=C7*2", &m, &no_names()), Err(Refusal::RowNotMatched));
}

/// Test 8. Half understood: one mappable reference and one refused construct. Refusal wins.
#[test]
fn a_half_understood_formula_is_refused_whole() {
    assert_eq!(map("=C5+Sheet2!D5"), None);
    assert_eq!(map("=C5+MyName"), None);
    assert_eq!(map("=MyName+C5"), None);
}

/// A defined name shaped like a cell reference refuses; the same formula with no such name declared maps. The
/// pair is the point: the names come from the file, so we do not rely on Excel's rule that such a name cannot exist.
#[test]
fn an_a1_shaped_defined_name_refuses_and_the_same_formula_without_it_maps() {
    let names = normalise_defined_names(["TAX7"]);
    assert_eq!(mr("=TAX7*C5", &insert_at_5(), &names), None);
    assert_eq!(
        an("=TAX7*C5", &insert_at_5(), &names),
        Err(Refusal::DefinedName)
    );
    assert_eq!(
        mr("=TAX7*C5", &insert_at_5(), &no_names()).as_deref(),
        Some("=TAX8*C6")
    );
}

/// Names match case-insensitively, by the same `to_lowercase` rule `meta.rs` applies.
#[test]
fn defined_names_match_case_insensitively() {
    let names = normalise_defined_names(["MyRate"]);
    assert_eq!(mr("=myrate*C5", &insert_at_5(), &names), None);
    assert_eq!(mr("=MYRATE*C5", &insert_at_5(), &names), None);
}

#[test]
fn further_refusals_the_proposal_added() {
    // Leading zero: the mapping is right and the text could not byte-match, because the zero would be lost.
    assert_eq!(why("=A05"), Refusal::A1ShapedOutOfRange);
    // Out of range.
    assert_eq!(why("=ZZZ1"), Refusal::A1ShapedOutOfRange);
    assert_eq!(why("=A0"), Refusal::A1ShapedOutOfRange);
    assert_eq!(why("=A1048577"), Refusal::A1ShapedOutOfRange);
    // A locale separator.
    assert_eq!(why("=SUM(C5;C6)"), Refusal::UnexpectedCharacter);
    // Whitespace before the paren does not hide a function name that reads as a reference.
    assert_eq!(why("=LOG10 (C5)"), Refusal::A1ShapedFunction);
    // `ATAN2` and `DEC2BIN` are NOT A1-shaped (a column has at most three letters, and `DEC2BIN` has letters after
    // its digit), so they are ordinary function names and pass through. `LOG10` is the one that reads as a cell.
    assert_eq!(map("=ATAN2(C5,1)").as_deref(), Some("=ATAN2(C6,1)"));
    assert_eq!(map("=DEC2BIN(C5)").as_deref(), Some("=DEC2BIN(C6)"));
    // R1C1-looking words are not recognised.
    assert_eq!(why("=R5C3"), Refusal::UnrecognisedWord);
    // Error literals, array constants, spill and implicit intersection.
    for f in ["=#REF!", "={1,2}", "=C5#", "=@C5"] {
        assert_eq!(map(f), None, "{f}");
    }
    // A number glued to a letter, and non-ASCII.
    assert_eq!(why("=5A"), Refusal::BadNumber);
    assert_eq!(why("=C5+é"), Refusal::NotAscii);
    // A range written backwards refuses, even under a monotone mapping.
    assert_eq!(why("=SUM(C10:C5)"), Refusal::BackwardsRange);
    // A `:` with no reference on one side.
    assert_eq!(why("=C5:SUM(1)"), Refusal::MisplacedColon);
}

// ---------------------------------------------------------------------------
// Mappings. Each must produce the exact expected text.
// ---------------------------------------------------------------------------

/// Test 9. Old row 5 to new row 6.
#[test]
fn a_reference_maps_through_the_row_mapping() {
    assert_eq!(map("=C5*2").as_deref(), Some("=C6*2"));
}

/// Test 10. A reference above the edit does not move, and is not refused. Also the direction check: the mapping
/// is old to new, so an old `C6` becomes `C7`, not `C5`. A backwards implementation passes test 9 only if the
/// mapping happens to be symmetric.
#[test]
fn a_reference_above_the_edit_stays_and_the_direction_is_old_to_new() {
    assert_eq!(map("=C3*2").as_deref(), Some("=C3*2"));
    assert_eq!(map("=C4+C5").as_deref(), Some("=C4+C6"));
    assert_eq!(map("=C6").as_deref(), Some("=C7"));
}

/// Test 11. `$` is preserved and the row still maps. `$` does not prevent a reference moving on insertion: it
/// governs copy and fill. So `$C$5` becomes `$C$6` when a row is inserted above it.
#[test]
fn dollar_signs_are_preserved_and_do_not_stop_the_row_moving() {
    assert_eq!(map("=$C$5").as_deref(), Some("=$C$6"));
    assert_eq!(map("=C$5").as_deref(), Some("=C$6"));
    assert_eq!(map("=$C5").as_deref(), Some("=$C6"));
}

/// Test 12. Both endpoints map.
#[test]
fn both_range_endpoints_map() {
    assert_eq!(map("=SUM(C5:C10)").as_deref(), Some("=SUM(C6:C11)"));
    assert_eq!(map("=SUM(C3:C10)").as_deref(), Some("=SUM(C3:C11)"));
    assert_eq!(map("=SUM($C$5:$C$10)").as_deref(), Some("=SUM($C$6:$C$11)"));
}

/// Test 13. Multi-digit and multi-letter.
#[test]
fn multi_digit_and_multi_letter_references() {
    assert_eq!(map("=AB123+AB7").as_deref(), Some("=AB124+AB8"));
    assert_eq!(map("=XFD1048576").as_deref(), None); // row not in this test's mapping
}

/// Test 14. Whitespace and case preserved exactly; only the row changes.
#[test]
fn whitespace_and_case_are_preserved() {
    assert_eq!(map("= c5 * 2").as_deref(), Some("= c6 * 2"));
    assert_eq!(
        map("=\tc5\r\n+ Sum( C5 ,\t$a$9 )").as_deref(),
        Some("=\tc6\r\n+ Sum( C6 ,\t$a$10 )")
    );
}

#[test]
fn numbers_booleans_and_function_names_pass_through_untouched() {
    for f in [
        "=1E5*2",
        "=.5+C3",
        "=1.5E+3",
        "=TRUE",
        "=false",
        "=SUM(1,2)",
        "=_xlfn.IFS(C3,1)",
        "=STDEV.S(C3:C4)",
        "=10%",
        "=-C3",
        "=C3<>C4",
        "=C3&C4",
    ] {
        assert!(map(f).is_some(), "{f} should map");
    }
    assert_eq!(map("=1E5*C5").as_deref(), Some("=1E5*C6"));
    assert_eq!(map("=_xlfn.IFS(C5,1)").as_deref(), Some("=_xlfn.IFS(C6,1)"));
    // An unknown function is passed through: we hold no function list and want none.
    assert_eq!(map("=MYFUNC(C5)").as_deref(), Some("=MYFUNC(C6)"));
}

// ---------------------------------------------------------------------------
// Properties
// ---------------------------------------------------------------------------

/// Test 15. Under an identity mapping every accepted formula comes back byte-identical. A violation means
/// normalisation crept in.
#[test]
fn identity_mapping_returns_every_accepted_formula_byte_identical() {
    let identity: BTreeMap<u32, u32> = (1..=300).map(|r| (r, r)).collect();
    let mut accepted = 0;
    let mut rng = Lcg(7);
    for _ in 0..2000 {
        let f = generate(&mut rng, true);
        if let Some(out) = mr(&f, &identity, &no_names()) {
            assert_eq!(out, f);
            accepted += 1;
        }
    }
    assert!(
        accepted > 1500,
        "only {accepted} of 2000 generated formulas were accepted"
    );
}

/// Test 16. Mapping then mapping through the inverse returns the input. `matched` is injective (clause 4 of
/// the row-alignment invariant, `docs/src/maintainers/row-alignment-invariant.md`), so the inverse exists and is
/// the reversed map. This is the first test that leans on that proof.
#[test]
fn mapping_then_inverse_mapping_returns_the_input() {
    let mut rng = Lcg(99);
    let mut checked = 0;
    for round in 0..40 {
        // Phase A: a strictly increasing injection, so ranges never run backwards and nothing is refused.
        // Phase B: an arbitrary injection, no ranges, so nothing is refused either.
        let monotone = round % 2 == 0;
        let forward = random_injection(&mut rng, 60, monotone);
        let inverse: BTreeMap<u32, u32> = forward.iter().map(|(&o, &n)| (n, o)).collect();
        assert_eq!(
            inverse.len(),
            forward.len(),
            "the mapping must be injective"
        );
        for _ in 0..100 {
            let f = generate(&mut rng, monotone);
            let mapped = mr(&f, &forward, &no_names())
                .unwrap_or_else(|| panic!("generated formula refused: {f}"));
            let back = mr(&mapped, &inverse, &no_names())
                .unwrap_or_else(|| panic!("mapped formula refused: {mapped}"));
            assert_eq!(back, f);
            checked += 1;
        }
    }
    assert_eq!(checked, 4000);
}

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// Rows 1..=`n` mapped into 1..=`2n`, injectively; strictly increasing when `monotone`.
fn random_injection(rng: &mut Lcg, n: u32, monotone: bool) -> BTreeMap<u32, u32> {
    let mut targets: Vec<u32> = (1..=2 * n).collect();
    for i in (1..targets.len()).rev() {
        targets.swap(i, rng.below(i as u64 + 1) as usize);
    }
    targets.truncate(n as usize);
    if monotone {
        targets.sort_unstable();
    }
    (1..=n).zip(targets).collect()
}

/// A formula from the accepted grammar, with references on rows 1..=60. Ranges only when `ranges`.
fn generate(rng: &mut Lcg, ranges: bool) -> String {
    const COLS: [&str; 5] = ["A", "C", "AB", "XFD", "z"];
    let reference = |rng: &mut Lcg| {
        let d1 = if rng.below(3) == 0 { "$" } else { "" };
        let d2 = if rng.below(3) == 0 { "$" } else { "" };
        format!(
            "{d1}{}{d2}{}",
            COLS[rng.below(5) as usize],
            1 + rng.below(60)
        )
    };
    let mut out = String::from("=");
    for n in 0..1 + rng.below(4) {
        if n > 0 {
            out.push_str(["+", " - ", "*", "&", "<>", ">="][rng.below(6) as usize]);
        }
        match rng.below(5) {
            0 => out.push_str(&reference(rng)),
            1 if ranges => {
                let a = 1 + rng.below(30);
                let b = a + rng.below(30);
                out.push_str(&format!("SUM(C{a}:C{b})"));
            }
            2 => out.push_str(&format!("MAX({}, {})", reference(rng), rng.below(1000))),
            3 => out.push_str(&format!(" {} ", reference(rng))),
            _ => out.push_str(["1E5", ".5", "TRUE", "10%"][rng.below(4) as usize]),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// What refusing costs, over real formulas
// ---------------------------------------------------------------------------

/// Every formula in every fixture workbook, mapped under a shift-by-one mapping, counted by outcome. The counts
/// are the evidence behind the "blunt on `"`" decision. The test asserts only that formulas were found, so it
/// cannot pass by finding none, and it prints the table (`cargo test … -- --nocapture`).
#[test]
fn what_refusing_costs_over_the_fixture_corpus() {
    use calamine::{Reader, open_workbook_auto};
    let mapping: BTreeMap<u32, u32> = (1..=20_000).map(|r| (r, r + 1)).collect();
    let mut outcomes: BTreeMap<String, usize> = BTreeMap::new();
    let mut total = 0;
    let mut stack = vec![std::path::PathBuf::from("tests/fixtures")];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "xlsx") {
                // Fixtures that exist to be malformed are not workbooks; skip those that do not open.
                let Ok(mut wb) = open_workbook_auto(&path) else {
                    continue;
                };
                for name in wb.sheet_names() {
                    let Ok(range) = wb.worksheet_formula(&name) else {
                        continue;
                    };
                    for (_, _, f) in range.cells() {
                        if f.is_empty() {
                            continue;
                        }
                        total += 1;
                        let key = match an(f, &mapping, &no_names()) {
                            Ok(_) => "mapped".to_string(),
                            Err(r) => format!("refused: {r:?}"),
                        };
                        *outcomes.entry(key).or_default() += 1;
                    }
                }
            }
        }
    }
    println!("formulas found: {total}");
    for (k, v) in &outcomes {
        println!("{v:>6}  {k}");
    }
    assert!(
        total > 0,
        "no formulas found: the measurement would be vacuous"
    );
}

// ---------------------------------------------------------------------------
// Ranges need a monotone mapping (architect review, 2026-10-06)
// ---------------------------------------------------------------------------

/// The measured reproduction, through the real alignment: `RowKey` on column 1 with one keyless row. The LCS gives
/// `{1->1, 3->2}`, which is monotone, and the keyless rescue adds `2->3`, which makes the union not monotone.
fn reproduction() -> BTreeMap<u32, u32> {
    use crate::diff::NormalizedCell;
    use crate::model::{CellValue, SheetRef};
    use crate::options::AlignmentMode;
    let cells = |data: &[(u32, u32, &str)]| -> crate::diff::CellMap {
        data.iter()
            .map(|&(r, c, v)| ((r, c), NormalizedCell::for_test(CellValue::Text(v.into()))))
            .collect()
    };
    let old = cells(&[(1, 1, "K1"), (2, 2, "spacer"), (3, 1, "K2")]);
    let new = cells(&[(1, 1, "K1"), (2, 1, "K2"), (3, 2, "spacer")]);
    let sheet = SheetRef {
        name: "Sheet1".into(),
        index: 0,
    };
    let mapping = crate::align::compute_row_mapping(
        &old,
        &new,
        &AlignmentMode::RowKey { columns: vec![1] },
        None,
        None,
        &sheet,
        &mut Vec::new(),
    )
    .unwrap()
    .unwrap();
    mapping.matched
}

#[test]
fn the_keyless_rescue_really_produces_a_non_monotone_mapping() {
    let m = reproduction();
    assert_eq!(m, BTreeMap::from([(1, 1), (2, 3), (3, 2)]));
    assert!(!RowMap::new(&m).monotone);
}

/// `C1:C2` would map to `C1:C3` by endpoints, which contains the image of old row 3. It must refuse.
#[test]
fn a_range_on_a_non_monotone_sheet_is_refused_even_when_it_would_not_look_backwards() {
    let m = reproduction();
    assert_eq!(
        an("=SUM(C1:C2)", &m, &no_names()),
        Err(Refusal::NonMonotoneMapping)
    );
    assert_eq!(
        an("=SUM(C2:C3)", &m, &no_names()),
        Err(Refusal::NonMonotoneMapping)
    );
}

/// The same range on a monotone sheet is mapped.
#[test]
fn the_same_range_on_a_monotone_sheet_is_mapped() {
    let m: BTreeMap<u32, u32> = [(1, 1), (2, 2), (3, 4)].into();
    assert!(RowMap::new(&m).monotone);
    assert_eq!(
        mr("=SUM(C1:C2)", &m, &no_names()).as_deref(),
        Some("=SUM(C1:C2)")
    );
    assert_eq!(
        mr("=SUM(C1:C3)", &m, &no_names()).as_deref(),
        Some("=SUM(C1:C4)")
    );
}

/// Single references are unaffected by non-monotonicity: each maps through `matched` on its own.
#[test]
fn a_single_reference_on_a_non_monotone_sheet_still_maps() {
    let m = reproduction();
    assert_eq!(mr("=C2+C3", &m, &no_names()).as_deref(), Some("=C3+C2"));
}
