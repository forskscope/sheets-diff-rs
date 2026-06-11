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
    let _new_remaining: Vec<&SheetRef> = old_sheets.iter().collect::<Vec<_>>();

    let mut matched_old_indices = vec![false; old_sheets.len()];
    let mut matched_new_indices = vec![false; new_sheets.len()];

    for (oi, old) in old_sheets.iter().enumerate() {
        if let Some(ni) = new_sheets.iter().position(|n| n.name == old.name) {
            let new = &new_sheets[ni];
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
            conservative_rename(
                &old_remaining,
                &new_remaining_refs,
                &mut pairs,
                diagnostics,
            );
        }

        SheetMatchingMode::ExactNameThenIndex => {
            index_match(&old_remaining, &mut new_remaining_refs, &mut pairs);
            // Remaining are Added / Removed
            let still_unmatched_old: Vec<&SheetRef> =
                old_remaining.iter().copied().filter(|s| {
                    !pairs.iter().any(|p| p.old_sheet.as_ref().map(|r| r.name == s.name).unwrap_or(false)
                        && !matches!(p.change, SheetChange::Removed))
                }).collect();
            let still_unmatched_new: Vec<&SheetRef> =
                new_remaining_refs.iter().copied().filter(|s| {
                    !pairs.iter().any(|p| p.new_sheet.as_ref().map(|r| r.name == s.name).unwrap_or(false)
                        && !matches!(p.change, SheetChange::Added))
                }).collect();
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
                        reason: SheetMatchReason::IndexAndContent,
                    },
                    MatchConfidence::Medium,
                )
            } else {
                (
                    SheetChange::RenamedAndMoved {
                        confidence: MatchConfidence::Low,
                        reason: SheetMatchReason::IndexAndContent,
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
        if let Some(ni) = new_remaining.iter().position(|n| n.index == old.index) {
            if !used_new[ni] {
                used_new[ni] = true;
                pairs.push(MatchedPair {
                    old_sheet: Some((*old).clone()),
                    new_sheet: Some(new_remaining[ni].clone()),
                    change: SheetChange::Renamed {
                        confidence: MatchConfidence::Low,
                        reason: SheetMatchReason::IndexAndContent,
                    },
                });
            }
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
        SheetRef { name: name.into(), index }
    }

    #[test]
    fn exact_name_match() {
        let old = vec![sref("Sheet1", 0)];
        let new = vec![sref("Sheet1", 0)];
        let mut diag = vec![];
        let pairs = match_sheets(&old, &new, SheetMatchingMode::ExactNameThenConservativeRename, &mut diag);
        assert_eq!(pairs.len(), 1);
        assert!(matches!(pairs[0].change, SheetChange::Unchanged | SheetChange::Moved));
        assert!(diag.is_empty());
    }

    #[test]
    fn added_sheet() {
        let old = vec![];
        let new = vec![sref("Sheet1", 0)];
        let mut diag = vec![];
        let pairs = match_sheets(&old, &new, SheetMatchingMode::ExactNameThenConservativeRename, &mut diag);
        assert_eq!(pairs.len(), 1);
        assert!(matches!(pairs[0].change, SheetChange::Added));
    }

    #[test]
    fn removed_sheet() {
        let old = vec![sref("Sheet1", 0)];
        let new = vec![];
        let mut diag = vec![];
        let pairs = match_sheets(&old, &new, SheetMatchingMode::ExactNameThenConservativeRename, &mut diag);
        assert_eq!(pairs.len(), 1);
        assert!(matches!(pairs[0].change, SheetChange::Removed));
    }

    #[test]
    fn single_rename_detected() {
        let old = vec![sref("OldName", 0)];
        let new = vec![sref("NewName", 0)];
        let mut diag = vec![];
        let pairs = match_sheets(&old, &new, SheetMatchingMode::ExactNameThenConservativeRename, &mut diag);
        assert_eq!(pairs.len(), 1);
        assert!(matches!(pairs[0].change, SheetChange::Renamed { .. }));
        assert!(diag.is_empty());
    }

    #[test]
    fn multiple_ambiguous_produce_diagnostic() {
        let old = vec![sref("A", 0), sref("B", 1)];
        let new = vec![sref("C", 0), sref("D", 1)];
        let mut diag = vec![];
        let pairs = match_sheets(&old, &new, SheetMatchingMode::ExactNameThenConservativeRename, &mut diag);
        // All become Added + Removed
        assert!(pairs.iter().all(|p| matches!(p.change, SheetChange::Added | SheetChange::Removed)));
        assert_eq!(diag.len(), 1);
        assert!(matches!(diag[0].kind, DiagnosticKind::AmbiguousSheetMatch { .. }));
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
}
