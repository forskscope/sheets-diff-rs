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
            // Remaining are Added / Removed. Checked by *identity* (`index`, unique per side —
            // see `SheetRef::index`'s doc comment and its sole construction site, `open.rs`'s
            // `.enumerate()` over one workbook's own sheets), not by name (f134): a leftover
            // sheet whose *name* happens to coincide with an unrelated already-matched pair's
            // name is a different sheet and must still be reported, not treated as accounted
            // for. `index_match` above already relies on the same per-side uniqueness.
            let still_unmatched_old: Vec<&SheetRef> = old_remaining
                .iter()
                .copied()
                .filter(|s| {
                    !pairs.iter().any(|p| {
                        p.old_sheet
                            .as_ref()
                            .map(|r| r.index == s.index)
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
                            .map(|r| r.index == s.index)
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
            let change = if old.index == new.index {
                SheetChange::Renamed {
                    confidence: MatchConfidence::Medium,
                    reason: SheetMatchReason::SameIndex,
                }
            } else {
                // Formed by elimination: the names differ, the indices differ, and
                // the two sheets are paired only because each is the sole unmatched
                // sheet on its side. No content is compared and no index matched, so
                // this is the weakest pairing the matcher makes — do not read it as
                // "the index matched" (the arm above) or as anything about content.
                SheetChange::RenamedAndMoved {
                    confidence: MatchConfidence::Low,
                    reason: SheetMatchReason::SoleRemainingPair,
                }
            };
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
mod tests;
