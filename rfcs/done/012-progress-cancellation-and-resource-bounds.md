# RFC-012: Progress, Cancellation, and Resource Bounds

**Status.** Implemented (2.0.0–2.2.3) — restored 2026-08-15; not individually re-verified against the implementation.
**Target:** v2.0.0 hooks, v2.x refinements  
**Created:** 2026-06-11  
**Category:** Integration  

## 1. Summary

Add progress events, cancellation, and resource limits suitable for GUI background jobs and batch safety.

## 2. Motivation

Spreadsheet comparison can be slow on large or pathological workbooks. GUI applications need to keep the UI responsive and allow users to cancel. Batch systems need bounds to avoid runaway jobs.

## 3. Goals

- Provide cancellation checks at major pipeline stages.
- Provide progress events suitable for UI status updates.
- Provide maximum sheets/cells/change-count bounds.
- Return structured cancellation and limit errors.
- Avoid requiring a specific async runtime.

## 4. Non-goals

- Do not make the comparison API async in v2.0.
- Do not guarantee exact progress percentages for all workbooks.
- Do not spawn threads internally by default.

## 5. External design

Options:

```rust
pub struct ProgressOptions {
    pub sink: Option<Box<dyn ProgressSink + Send>>,
    pub cancellation: Option<Box<dyn Cancellation + Send + Sync>>,
}

pub trait ProgressSink {
    fn on_event(&mut self, event: DiffEvent);
}

pub trait Cancellation {
    fn is_cancelled(&self) -> bool;
}
```

Events:

```rust
pub enum DiffEvent {
    Started,
    OpeningWorkbook { side: Side },
    ReadingSheet { side: Side, sheet: SheetRef },
    MatchingSheets,
    ComparingSheet { sheet: SheetPairLabel },
    ComparedCells { current: u64, total_hint: Option<u64> },
    Finished,
}
```

## 6. Internal design

Internal implementation should call `check_cancel()` before and during expensive loops. Progress emission should be throttled to avoid calling the sink for every cell unless requested.

Bounds:

```rust
pub struct Bounds {
    pub max_sheets: Option<usize>,
    pub max_compared_cells: Option<u64>,
    pub max_cell_diffs: Option<u64>,
}
```

## 7. Data lifecycle

1. Options are validated.
2. `Started` event emitted.
3. Cancellation checked before opening each workbook.
4. Sheet-level progress emitted during reading and comparison.
5. Cell-loop progress emitted periodically.
6. Bounds checked as counters increase.
7. `Finished` emitted only for successful completion.

## 8. Error, diagnostic, and edge-case behavior

Cancellation returns `SheetsDiffError::Cancelled`. Bounds return `SheetsDiffError::LimitExceeded` with the exceeded limit kind.

Progress sink errors should not exist in the base trait. If caller code can fail, it should store failure and request cancellation.

## 9. Testing and acceptance criteria

Acceptance criteria:

- A cancellation predicate stops comparison before completion.
- Max cell bound stops a large comparison.
- Progress events occur in documented order.
- Default API works without progress/cancellation objects.
- No async runtime is required.

## 10. Migration and compatibility

Existing callers do not need to use events. GUI callers can pass adapters from their own cancellation token/job progress model.

## 11. Open questions

- Should the progress sink be `FnMut(DiffEvent)` instead of a trait?
- Should cancellation be an `Arc<AtomicBool>` convenience in addition to a trait?
