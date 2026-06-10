use std::fmt;

#[cfg(feature = "serde_derive")]
use serde::{Deserialize, Serialize};

use super::diff::Diff;

/// Unified diff ready for further formatting or splitting.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_derive", derive(Serialize, Deserialize))]
pub struct UnifiedDiff {
    pub content: Vec<UnifiedDiffContent>,
}

/// Unified diff with prefix markers applied to each line.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_derive", derive(Serialize, Deserialize))]
pub struct FormattedUnifiedDiff {
    pub content: Vec<UnifiedDiffContent>,
}

/// One sheet's worth of unified diff content.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_derive", derive(Serialize, Deserialize))]
pub struct UnifiedDiffContent {
    pub old_title: String,
    pub new_title: String,
    pub lines: Vec<UnifiedDiffLine>,
}

/// A single line in a unified diff.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_derive", derive(Serialize, Deserialize))]
pub struct UnifiedDiffLine {
    pub pos: Option<String>,
    pub old: Option<String>,
    pub new: Option<String>,
}

/// Unified diff split into separate old and new views.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_derive", derive(Serialize, Deserialize))]
pub struct SplitUnifiedDiff {
    pub old: Vec<SplitUnifiedDiffContent>,
    pub new: Vec<SplitUnifiedDiffContent>,
}

/// One sheet's content in a split unified diff.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_derive", derive(Serialize, Deserialize))]
pub struct SplitUnifiedDiffContent {
    pub title: String,
    pub lines: Vec<SplitUnifiedDiffLine>,
}

/// A single line in a split unified diff.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde_derive", derive(Serialize, Deserialize))]
pub struct SplitUnifiedDiffLine {
    pub pos: Option<String>,
    pub text: Option<String>,
}

impl UnifiedDiff {
    /// Applies `---`/`+++`/`-`/`+`/`@@` prefix markers to all fields.
    pub fn format(&self) -> FormattedUnifiedDiff {
        let content = self
            .content
            .iter()
            .map(|x| {
                let old_title = format!("--- {}", &x.old_title);
                let new_title = format!("+++ {}", &x.new_title);

                let lines = x
                    .lines
                    .iter()
                    .map(|x| UnifiedDiffLine {
                        pos: x.pos.as_ref().map(|pos| format!("@@ {} @@", pos)),
                        old: x.old.as_ref().map(|old| format!("- {}", old)),
                        new: x.new.as_ref().map(|new| format!("+ {}", new)),
                    })
                    .collect();

                UnifiedDiffContent { old_title, new_title, lines }
            })
            .collect();
        FormattedUnifiedDiff { content }
    }

    /// Splits into separate old and new content views.
    pub fn split(&self) -> SplitUnifiedDiff {
        let old = self
            .content
            .iter()
            .map(|x| SplitUnifiedDiffContent {
                title: x.old_title.clone(),
                lines: x
                    .lines
                    .iter()
                    .map(|x| SplitUnifiedDiffLine {
                        pos: x.pos.as_ref().map(|p| p.to_owned()),
                        text: x.old.as_ref().map(|t| t.to_owned()),
                    })
                    .collect(),
            })
            .collect();

        let new = self
            .content
            .iter()
            .map(|x| SplitUnifiedDiffContent {
                title: x.new_title.clone(),
                lines: x
                    .lines
                    .iter()
                    .map(|x| SplitUnifiedDiffLine {
                        pos: x.pos.as_ref().map(|p| p.to_owned()),
                        text: x.new.as_ref().map(|t| t.to_owned()),
                    })
                    .collect(),
            })
            .collect();

        SplitUnifiedDiff { old, new }
    }
}

impl fmt::Display for FormattedUnifiedDiff {
    /// Renders the unified diff as a plain-text string.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for x in &self.content {
            writeln!(f, "{}", &x.old_title)?;
            writeln!(f, "{}", &x.new_title)?;
            for x in &x.lines {
                if let Some(pos) = &x.pos {
                    writeln!(f, "{}", pos)?;
                }
                if let Some(old) = &x.old {
                    writeln!(f, "{}", old)?;
                }
                if let Some(new) = &x.new {
                    writeln!(f, "{}", new)?;
                }
            }
        }
        Ok(())
    }
}

/// Builds a [`UnifiedDiff`] from the raw [`Diff`] result.
pub fn unified_diff(diff: &Diff) -> UnifiedDiff {
    let mut ret: Vec<UnifiedDiffContent> = vec![];

    if !diff.sheet_diff.is_empty() {
        let lines = diff
            .sheet_diff
            .iter()
            .map(|x| UnifiedDiffLine {
                pos: None,
                old: x.old.clone(),
                new: x.new.clone(),
            })
            .collect();

        ret.push(UnifiedDiffContent {
            old_title: format!("{} (sheet names)", diff.old_filepath),
            new_title: format!("{} (sheet names)", diff.new_filepath),
            lines,
        });
    }

    let cell_diffs_content: Vec<UnifiedDiffContent> = diff
        .cell_diffs
        .iter()
        .map(|x| {
            let lines = x
                .cells
                .iter()
                .map(|x| UnifiedDiffLine {
                    pos: Some(format!("{}({},{}) {}", x.addr, x.row, x.col, x.kind)),
                    old: x.old.clone(),
                    new: x.new.clone(),
                })
                .collect();

            UnifiedDiffContent {
                old_title: format!("{} [{}]", diff.old_filepath, x.sheet),
                new_title: format!("{} [{}]", diff.new_filepath, x.sheet),
                lines,
            }
        })
        .collect();

    ret.extend(cell_diffs_content);

    UnifiedDiff { content: ret }
}
