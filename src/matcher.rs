//! Sheet matching: exact-name pairing and conservative rename detection (RFC-009).

use crate::model::{
    Diagnostic, DiagnosticKind, DiagnosticLocation, DiffStage, MatchConfidence, Severity,
    SheetChange, SheetMatchReason, SheetRef,
};
use crate::options::SheetMatchingMode;

// ---------------------------------------------------------------------------
// Matched pair
// ---------------------------------------------------------------------------

/// The result of matching one logical sheet pair.
#[derive(Debug)]
pub struct MatchedPair {
    pub old_sheet: Option<SheetRef>,
    pub new_sheet: Option<SheetRef>,
    pub change: SheetChange,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Match the sheets from two workbooks under the given mode.
///
/// Returns the list of matched/unmatched pairs and any ambiguity diagnostics.
pub fn match_sheets(
    old_sheets: &[SheetRef],
    new_sheets: &[SheetRef],
    mode: SheetMatchingMode,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<MatchedPair> {
    let mut pairs: Vec<MatchedPair> = Vec::new();

    // Phase 1: exact name matching
    let mut old_remaining: Vec<&SheetRef> = Vec::new();

    let mut matched_old_indices = vec![false; old_sheets.len()];
    let mut matched_new_indices = vec![false; new_sheets.len()];

    for (oi, old) in old_sheets.iter().enumerate() {
        // `matched_new_indices` must be consulted *here*, not only recorded below (f133): two old
        // sheets sharing a name must not both claim the same first-appearing new sheet of that name.
        // `find` walks the new sheets in order and stops at the first one both same-named and not
        // yet claimed, so with duplicates on both sides the k-th old sheet of a name pairs with the
        // k-th new sheet of it — the same "order among the duplicates is the only signal, and index
        // matching already assumes it" reasoning `index_match` below already relies on. An old sheet
        // that runs out of same-named new sheets to claim (more old than new, for that name) finds
        // none here and falls through to `old_remaining`, reaching the later phases like any other
        // unmatched sheet — reported `Removed` if nothing else claims it, never silently dropped and
        // never paired with an unrelated name.
        if let Some((ni, new)) = new_sheets
            .iter()
            .enumerate()
            .find(|(ni, n)| !matched_new_indices[*ni] && n.name == old.name)
        {
            let change = if old.index == new.index {
                SheetChange::Unchanged // may be upgraded to Modified after cell diff
            } else {
                SheetChange::Moved
            };
            pairs.push(MatchedPair {
                old_sheet: Some(old.clone()),
                new_sheet: Some(new.clone()),
                change,
            });
            matched_old_indices[oi] = true;
            matched_new_indices[ni] = true;
        }
    }

    // Collect unmatched sheets
    for (i, old) in old_sheets.iter().enumerate() {
        if !matched_old_indices[i] {
            old_remaining.push(old);
        }
    }
    let mut new_remaining_refs: Vec<&SheetRef> = new_sheets
        .iter()
        .enumerate()
        .filter(|(i, _)| !matched_new_indices[*i])
        .map(|(_, s)| s)
        .collect();

    // Phase 2: rename detection
    match mode {
        SheetMatchingMode::ExactNameOnly => {
            // Mark remaining as Added / Removed
            push_removed(&old_remaining, &mut pairs);
            push_added(&new_remaining_refs, &mut pairs);
        }

        SheetMatchingMode::ExactNameThenConservativeRename => {
            conservative_rename(&old_remaining, &new_remaining_refs, &mut pairs, diagnostics);
        }

        SheetMatchingMode::ExactNameThenIndex => {
            index_match(&old_remaining, &mut new_remaining_refs, &mut pairs);
            // Remaining are Added / Removed
            let still_unmatched_old: Vec<&SheetRef> = old_remaining
                .iter()
                .copied()
                .filter(|s| {
                    !pairs.iter().any(|p| {
                        p.old_sheet
                            .as_ref()
                            .map(|r| r.name == s.name)
                            .unwrap_or(false)
                            && !matches!(p.change, SheetChange::Removed)
                    })
                })
                .collect();
            let still_unmatched_new: Vec<&SheetRef> = new_remaining_refs
                .iter()
                .copied()
                .filter(|s| {
                    !pairs.iter().any(|p| {
                        p.new_sheet
                            .as_ref()
                            .map(|r| r.name == s.name)
                            .unwrap_or(false)
                            && !matches!(p.change, SheetChange::Added)
                    })
                })
                .collect();
            push_removed(&still_unmatched_old, &mut pairs);
            push_added(&still_unmatched_new, &mut pairs);
        }
    }

    pairs
}

// ---------------------------------------------------------------------------
// Conservative rename detection
// ---------------------------------------------------------------------------

fn conservative_rename(
    old_remaining: &[&SheetRef],
    new_remaining: &[&SheetRef],
    pairs: &mut Vec<MatchedPair>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match (old_remaining.len(), new_remaining.len()) {
        (0, 0) => {}

        // Exactly one unmatched on each side — conservative rename candidate.
        (1, 1) => {
            let old = old_remaining[0];
            let new = new_remaining[0];
            let (change, confidence) = if old.index == new.index {
                (
                    SheetChange::Renamed {
                        confidence: MatchConfidence::Medium,
                        reason: SheetMatchReason::SameIndex,
                    },
                    MatchConfidence::Medium,
                )
            } else {
                // Formed by elimination: the names differ, the indices differ, and
                // the two sheets are paired only because each is the sole unmatched
                // sheet on its side. No content is compared and no index matched, so
                // this is the weakest pairing the matcher makes — do not read it as
                // "the index matched" (the arm above) or as anything about content.
                (
                    SheetChange::RenamedAndMoved {
                        confidence: MatchConfidence::Low,
                        reason: SheetMatchReason::SoleRemainingPair,
                    },
                    MatchConfidence::Low,
                )
            };
            let _ = confidence; // used inside change arms
            pairs.push(MatchedPair {
                old_sheet: Some(old.clone()),
                new_sheet: Some(new.clone()),
                change,
            });
        }

        // Multiple ambiguous candidates — leave as Added/Removed, emit diagnostic.
        _ => {
            let candidates: Vec<_> = new_remaining.iter().map(|s| (*s).clone()).collect();
            let candidates2: Vec<_> = old_remaining.iter().map(|s| (*s).clone()).collect();
            if !candidates.is_empty() && !candidates2.is_empty() {
                diagnostics.push(Diagnostic {
                    severity: Severity::Warning,
                    kind: DiagnosticKind::AmbiguousSheetMatch { candidates },
                    location: DiagnosticLocation {
                        stage: DiffStage::Match,
                        sheet_order: None,
                        sheet_name: None,
                        address: None,
                    },
                    message: format!(
                        "{} removed and {} added sheets could not be confidently matched; \
                         treating all as Added/Removed",
                        candidates2.len(),
                        new_remaining.len()
                    ),
                });
            }
            push_removed(old_remaining, pairs);
            push_added(new_remaining, pairs);
        }
    }
}

// ---------------------------------------------------------------------------
// Index-based fallback matching
// ---------------------------------------------------------------------------

fn index_match(
    old_remaining: &[&SheetRef],
    new_remaining: &mut Vec<&SheetRef>,
    pairs: &mut Vec<MatchedPair>,
) {
    let mut used_new = vec![false; new_remaining.len()];
    for old in old_remaining {
        if let Some(ni) = new_remaining.iter().position(|n| n.index == old.index)
            && !used_new[ni]
        {
            used_new[ni] = true;
            pairs.push(MatchedPair {
                old_sheet: Some((*old).clone()),
                new_sheet: Some(new_remaining[ni].clone()),
                change: SheetChange::Renamed {
                    confidence: MatchConfidence::Low,
                    reason: SheetMatchReason::SameIndex,
                },
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Simple Added / Removed push helpers
// ---------------------------------------------------------------------------

fn push_removed(sheets: &[&SheetRef], pairs: &mut Vec<MatchedPair>) {
    for s in sheets {
        pairs.push(MatchedPair {
            old_sheet: Some((*s).clone()),
            new_sheet: None,
            change: SheetChange::Removed,
        });
    }
}

fn push_added(sheets: &[&SheetRef], pairs: &mut Vec<MatchedPair>) {
    for s in sheets {
        pairs.push(MatchedPair {
            old_sheet: None,
            new_sheet: Some((*s).clone()),
            change: SheetChange::Added,
        });
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sref(name: &str, index: usize) -> SheetRef {
        SheetRef {
            name: name.into(),
            index,
        }
    }

    #[test]
    fn exact_name_match() {
        let old = vec![sref("Sheet1", 0)];
        let new = vec![sref("Sheet1", 0)];
        let mut diag = vec![];
        let pairs = match_sheets(
            &old,
            &new,
            SheetMatchingMode::ExactNameThenConservativeRename,
            &mut diag,
        );
        assert_eq!(pairs.len(), 1);
        assert!(matches!(
            pairs[0].change,
            SheetChange::Unchanged | SheetChange::Moved
        ));
        assert!(diag.is_empty());
    }

    #[test]
    fn added_sheet() {
        let old = vec![];
        let new = vec![sref("Sheet1", 0)];
        let mut diag = vec![];
        let pairs = match_sheets(
            &old,
            &new,
            SheetMatchingMode::ExactNameThenConservativeRename,
            &mut diag,
        );
        assert_eq!(pairs.len(), 1);
        assert!(matches!(pairs[0].change, SheetChange::Added));
    }

    #[test]
    fn removed_sheet() {
        let old = vec![sref("Sheet1", 0)];
        let new = vec![];
        let mut diag = vec![];
        let pairs = match_sheets(
            &old,
            &new,
            SheetMatchingMode::ExactNameThenConservativeRename,
            &mut diag,
        );
        assert_eq!(pairs.len(), 1);
        assert!(matches!(pairs[0].change, SheetChange::Removed));
    }

    /// The reason a pair carries, or panics if the pair is not a rename.
    fn reason_of(pair: &MatchedPair) -> &SheetMatchReason {
        match &pair.change {
            SheetChange::Renamed { reason, .. } | SheetChange::RenamedAndMoved { reason, .. } => {
                reason
            }
            other => panic!("not a rename: {other:?}"),
        }
    }

    // One test per site that constructs a reason (M10 unit 01). The three tests
    // below are the only places the matcher forms a rename.

    /// `conservative_rename`, one unmatched sheet a side, **equal** indices.
    #[test]
    fn reason_site_conservative_equal_index_is_same_index() {
        let pairs = match_sheets(
            &[sref("Old", 2)],
            &[sref("New", 2)],
            SheetMatchingMode::ExactNameThenConservativeRename,
            &mut vec![],
        );
        assert!(matches!(pairs[0].change, SheetChange::Renamed { .. }));
        assert_eq!(*reason_of(&pairs[0]), SheetMatchReason::SameIndex);
    }

    /// `conservative_rename`, one unmatched sheet a side, **unequal** indices: paired
    /// by elimination, the case that had no true reason before.
    #[test]
    fn reason_site_conservative_unequal_index_is_sole_remaining_pair() {
        let pairs = match_sheets(
            &[sref("Old", 0)],
            &[sref("New", 3)],
            SheetMatchingMode::ExactNameThenConservativeRename,
            &mut vec![],
        );
        assert!(matches!(
            pairs[0].change,
            SheetChange::RenamedAndMoved { .. }
        ));
        assert_eq!(*reason_of(&pairs[0]), SheetMatchReason::SoleRemainingPair);
    }

    /// `index_match` (`ExactNameThenIndex`): paired on `n.index == old.index`.
    #[test]
    fn reason_site_index_mode_is_same_index() {
        let pairs = match_sheets(
            &[sref("A", 0), sref("B", 1)],
            &[sref("X", 0), sref("Y", 1)],
            SheetMatchingMode::ExactNameThenIndex,
            &mut vec![],
        );
        let renames: Vec<_> = pairs
            .iter()
            .filter(|p| matches!(p.change, SheetChange::Renamed { .. }))
            .collect();
        assert_eq!(renames.len(), 2);
        for p in renames {
            assert_eq!(*reason_of(p), SheetMatchReason::SameIndex);
        }
    }

    #[test]
    fn single_rename_detected() {
        let old = vec![sref("OldName", 0)];
        let new = vec![sref("NewName", 0)];
        let mut diag = vec![];
        let pairs = match_sheets(
            &old,
            &new,
            SheetMatchingMode::ExactNameThenConservativeRename,
            &mut diag,
        );
        assert_eq!(pairs.len(), 1);
        assert!(matches!(pairs[0].change, SheetChange::Renamed { .. }));
        assert!(diag.is_empty());
    }

    #[test]
    fn multiple_ambiguous_produce_diagnostic() {
        let old = vec![sref("A", 0), sref("B", 1)];
        let new = vec![sref("C", 0), sref("D", 1)];
        let mut diag = vec![];
        let pairs = match_sheets(
            &old,
            &new,
            SheetMatchingMode::ExactNameThenConservativeRename,
            &mut diag,
        );
        // All become Added + Removed
        assert!(
            pairs
                .iter()
                .all(|p| matches!(p.change, SheetChange::Added | SheetChange::Removed))
        );
        assert_eq!(diag.len(), 1);
        assert!(matches!(
            diag[0].kind,
            DiagnosticKind::AmbiguousSheetMatch { .. }
        ));
    }

    #[test]
    fn exact_name_only_mode_does_not_rename() {
        let old = vec![sref("OldName", 0)];
        let new = vec![sref("NewName", 0)];
        let mut diag = vec![];
        let pairs = match_sheets(&old, &new, SheetMatchingMode::ExactNameOnly, &mut diag);
        assert_eq!(pairs.len(), 2); // Removed + Added
        assert!(diag.is_empty());
    }

    // -----------------------------------------------------------------------
    // f133 — two old sheets must not both claim the same new sheet
    // -----------------------------------------------------------------------

    /// The invariant the defect broke, stated on its own: no new-sheet index appears in more than
    /// one returned pair. Checked over every fixture below rather than trusted from their individual
    /// assertions, since the defect's own symptom was a duplicate *and* a silent drop at once — a
    /// count of pairs alone would not have caught it.
    fn assert_no_new_sheet_claimed_twice(pairs: &[MatchedPair]) {
        let mut seen: Vec<&SheetRef> = Vec::new();
        for p in pairs {
            if let Some(new) = &p.new_sheet {
                assert!(
                    !seen
                        .iter()
                        .any(|s| s.index == new.index && s.name == new.name),
                    "new sheet {new:?} appears in more than one pair: {pairs:?}"
                );
                seen.push(new);
            }
        }
    }

    /// The defect's exact shape, constructed rather than corrupted: two old sheets and two new
    /// sheets, all four sharing one name. Before the fix, `position` gave both old sheets the first
    /// new sheet — one pair used twice, the second new sheet in none. After: each old sheet pairs
    /// with the new sheet at the same position among the duplicates (index order is the only signal
    /// duplicate names leave, the same assumption `index_match` above already makes explicit).
    #[test]
    fn symmetric_duplicate_names_pair_in_order_not_the_same_sheet_twice() {
        let old = vec![sref("Sheet", 0), sref("Sheet", 1)];
        let new = vec![sref("Sheet", 0), sref("Sheet", 1)];
        let mut diag = vec![];
        let pairs = match_sheets(
            &old,
            &new,
            SheetMatchingMode::ExactNameThenConservativeRename,
            &mut diag,
        );
        assert_eq!(pairs.len(), 2, "{pairs:?}");
        assert!(
            pairs
                .iter()
                .all(|p| matches!(p.change, SheetChange::Unchanged)),
            "same names at the same indices on both sides: nothing moved: {pairs:?}"
        );
        assert_no_new_sheet_claimed_twice(&pairs);
        assert!(diag.is_empty());
    }

    /// The same shape with the second old/new pair's indices swapped between sides, so pairing by
    /// position among the duplicates (not by index equality) is what the test actually exercises:
    /// if the fix instead matched by index, this would come out wrong.
    #[test]
    fn symmetric_duplicate_names_pair_by_order_of_appearance_not_by_index() {
        // First "Sheet" on the old side is at index 0; on the new side, the first "Sheet" (by
        // position in the slice, which is what `iter()` walks) is at index 5.
        let old = vec![sref("Sheet", 0), sref("Sheet", 2)];
        let new = vec![sref("Sheet", 5), sref("Sheet", 9)];
        let mut diag = vec![];
        let pairs = match_sheets(
            &old,
            &new,
            SheetMatchingMode::ExactNameThenConservativeRename,
            &mut diag,
        );
        assert_eq!(pairs.len(), 2, "{pairs:?}");
        assert_no_new_sheet_claimed_twice(&pairs);
        // First old (index 0) paired with first new by appearance (index 5); second old (index 2)
        // with second new (index 9) — not matched by equal index, since none are equal.
        assert_eq!(pairs[0].old_sheet.as_ref().unwrap().index, 0);
        assert_eq!(pairs[0].new_sheet.as_ref().unwrap().index, 5);
        assert_eq!(pairs[1].old_sheet.as_ref().unwrap().index, 2);
        assert_eq!(pairs[1].new_sheet.as_ref().unwrap().index, 9);
    }

    /// The asymmetric case the handoff asks for by name: two old sheets share a name, only one new
    /// sheet has it. The first old sheet claims the new one; the second finds none left, falls
    /// through to `old_remaining`, and — nothing else being unmatched — is reported `Removed`. Not
    /// silently dropped (it is in `pairs`), not paired with something unrelated (there is nothing
    /// unrelated here to be paired with).
    #[test]
    fn asymmetric_duplicate_old_sheets_the_leftover_is_removed_not_dropped_or_misp_paired() {
        let old = vec![sref("Sheet", 0), sref("Sheet", 1)];
        let new = vec![sref("Sheet", 0)];
        let mut diag = vec![];
        let pairs = match_sheets(
            &old,
            &new,
            SheetMatchingMode::ExactNameThenConservativeRename,
            &mut diag,
        );
        assert_eq!(pairs.len(), 2, "{pairs:?}"); // one pair, one Removed — not one pair alone
        assert_no_new_sheet_claimed_twice(&pairs);
        let matched: Vec<_> = pairs
            .iter()
            .filter(|p| matches!(p.change, SheetChange::Unchanged))
            .collect();
        let removed: Vec<_> = pairs
            .iter()
            .filter(|p| matches!(p.change, SheetChange::Removed))
            .collect();
        assert_eq!(matched.len(), 1, "{pairs:?}");
        assert_eq!(removed.len(), 1, "{pairs:?}");
        assert_eq!(removed[0].old_sheet.as_ref().unwrap().index, 1);
        assert!(
            diag.is_empty(),
            "no ambiguous *candidate* exists on the new side: {diag:?}"
        );
    }

    /// The same asymmetric leftover, but with an unrelated added sheet also present: the leftover
    /// old sheet and the unrelated new sheet are now each the sole remaining unmatched sheet on
    /// their side, so `conservative_rename`'s existing (1, 1) arm treats them as a rename by
    /// elimination — exactly the same "sole remaining pair" rule already tested above for
    /// non-duplicate inputs (`reason_site_conservative_unequal_index_is_sole_remaining_pair`). Not a
    /// new behaviour this fix introduces; stated here so it is not mistaken for one.
    #[test]
    fn asymmetric_leftover_with_an_unrelated_sheet_is_the_existing_sole_remaining_pair_rule() {
        let old = vec![sref("Sheet", 0), sref("Sheet", 1)];
        let new = vec![sref("Sheet", 0), sref("Unrelated", 7)];
        let mut diag = vec![];
        let pairs = match_sheets(
            &old,
            &new,
            SheetMatchingMode::ExactNameThenConservativeRename,
            &mut diag,
        );
        assert_no_new_sheet_claimed_twice(&pairs);
        assert_eq!(pairs.len(), 2, "{pairs:?}");
        let renamed = pairs
            .iter()
            .find(|p| matches!(p.change, SheetChange::RenamedAndMoved { .. }))
            .unwrap_or_else(|| panic!("expected a sole-remaining-pair rename: {pairs:?}"));
        assert_eq!(renamed.old_sheet.as_ref().unwrap().index, 1);
        assert_eq!(renamed.new_sheet.as_ref().unwrap().name, "Unrelated");
    }

    /// `ExactNameOnly` does not run `conservative_rename` at all, so the asymmetric leftover has no
    /// chance to be mistaken for a rename regardless of what else is unmatched: always `Removed`.
    #[test]
    fn asymmetric_duplicate_under_exact_name_only_is_always_removed() {
        let old = vec![sref("Sheet", 0), sref("Sheet", 1)];
        let new = vec![sref("Sheet", 0), sref("Unrelated", 7)];
        let mut diag = vec![];
        let pairs = match_sheets(&old, &new, SheetMatchingMode::ExactNameOnly, &mut diag);
        assert_no_new_sheet_claimed_twice(&pairs);
        let removed: Vec<_> = pairs
            .iter()
            .filter(|p| matches!(p.change, SheetChange::Removed))
            .collect();
        assert_eq!(removed.len(), 1, "{pairs:?}");
        assert_eq!(removed[0].old_sheet.as_ref().unwrap().index, 1);
    }

    /// `ExactNameThenIndex` already guards its own claim (`index_match`'s `used_new`); this pins
    /// that the exact-name phase's fix does not disturb it when the two phases interact.
    ///
    /// The leftover old sheet is given a name distinct from the matched pair's on purpose — see
    /// `an_unrelated_matched_pairs_name_must_not_hide_a_leftover_sheet` just below for why sharing
    /// a name here would test a *different*, pre-existing defect instead of this one.
    #[test]
    fn duplicate_names_then_index_fallback_still_claims_each_new_sheet_once() {
        let old = vec![sref("Sheet", 0), sref("Sheet", 1), sref("Leftover", 2)];
        let new = vec![sref("Sheet", 0), sref("Sheet", 1)];
        let mut diag = vec![];
        let pairs = match_sheets(&old, &new, SheetMatchingMode::ExactNameThenIndex, &mut diag);
        assert_no_new_sheet_claimed_twice(&pairs);
        // Two claimed by exact name (index 0 and 1 on both sides); the third old sheet has no new
        // sheet left at all, by name or by index — Removed.
        let removed: Vec<_> = pairs
            .iter()
            .filter(|p| matches!(p.change, SheetChange::Removed))
            .collect();
        assert_eq!(removed.len(), 1, "{pairs:?}");
        assert_eq!(removed[0].old_sheet.as_ref().unwrap().index, 2);
    }

    /// **Not this unit's defect — reported in the review request, reproduced here to pin the
    /// current (wrong) behaviour rather than leave it undocumented.** `ExactNameThenIndex`'s own
    /// "still unmatched" filter (`match_sheets`, the arm's `still_unmatched_old`/`_new`) checks
    /// whether *any* pair shares the candidate's **name**, not whether it *is* the candidate. A
    /// leftover old sheet whose name coincides with an unrelated, already-matched pair's name is
    /// therefore excluded from `still_unmatched_old` and never reaches `push_removed` — silently
    /// dropped from the result entirely (not `Removed`, not anywhere), even though it is a
    /// completely different sheet. `index_match` itself is not at fault (it uses `used_new`
    /// correctly, per `duplicate_names_then_index_fallback_still_claims_each_new_sheet_once` above);
    /// the bug is in the name-based re-check that follows it. Out of this unit's scope
    /// (`index_match`/its surrounding phase, not the exact-name phase) — Non-change scope says
    /// report, not fix.
    #[test]
    fn known_defect_an_unrelated_matched_pairs_name_hides_a_leftover_sheet() {
        let old = vec![sref("Sheet", 0), sref("Sheet", 1)];
        let new = vec![sref("Sheet", 0)];
        let mut diag = vec![];
        let pairs = match_sheets(&old, &new, SheetMatchingMode::ExactNameThenIndex, &mut diag);
        // What SHOULD happen: old[1] is Removed, like it is under ExactNameOnly and
        // ExactNameThenConservativeRename for the same shape (see the asymmetric tests above).
        // What ACTUALLY happens: old[1] is dropped from `pairs` entirely, because old[0]'s matched
        // pair shares its name ("Sheet") and the filter does not check *which* sheet it is.
        assert_eq!(
            pairs.len(),
            1,
            "known defect: expected 2 (one Unchanged, one Removed), got {pairs:?} — a sheet vanished"
        );
    }
}
