//! `CellAddress` and `ComparedRange` as a downstream caller sees them (M10 unit 09).
//!
//! Both are `#[non_exhaustive]`, so from outside the crate — which is where this file is — a struct *literal* does not
//! compile. What survives, and is asserted here: **construction** by the public constructors (`CellAddress::new`,
//! `ComparedRange::empty` / `::union`), **reading** every field, **assignment** to the public fields of a value you own,
//! and getting them out of a real result. A caller is not locked out of a type they need to make.

use sheets_diff::{CellAddress, ComparedRange, MAX_COL, MAX_ROW, compare_bytes};

mod support;
use support::wb_strings;

#[test]
fn a_cell_address_is_made_by_new_and_its_fields_are_readable() {
    let a = CellAddress::new(3, 28).expect("in range");
    assert_eq!((a.row, a.col), (3, 28));
    assert_eq!(a.a1, "AB3");
    assert_eq!(a.to_string(), "AB3");
    // the constructor validates, so the three fields cannot disagree
    assert!(CellAddress::new(0, 1).is_none());
    assert!(CellAddress::new(1, 0).is_none());
    assert!(CellAddress::new(MAX_ROW + 1, 1).is_none());
    assert!(CellAddress::new(1, MAX_COL + 1).is_none());
    let last = CellAddress::new(MAX_ROW, MAX_COL).unwrap();
    assert_eq!(last.a1, "XFD1048576");
}

#[test]
fn a_compared_range_is_made_by_the_public_constructors_and_read_and_assigned() {
    let r = ComparedRange::union(Some((1, 1)), Some((2, 3)), Some((2, 2)), Some((5, 4)));
    assert_eq!((r.start, r.end), (Some((1, 1)), Some((5, 4))));
    let mut e = ComparedRange::empty();
    assert_eq!((e.start, e.end), (None, None));
    // assignment to the public fields of an owned value still works
    e.start = Some((2, 2));
    e.end = Some((3, 3));
    assert_eq!((e.start, e.end), (Some((2, 2)), Some((3, 3))));
    assert_eq!(
        e,
        ComparedRange::union(Some((2, 2)), Some((3, 3)), None, None)
    );
}

#[test]
fn both_come_out_of_a_real_result() {
    let old = wb_strings(&[(0, 0, "a"), (2, 1, "b")]);
    let new = wb_strings(&[(0, 0, "A"), (2, 1, "b")]);
    let d = compare_bytes(&old, &new).unwrap();
    let range: &ComparedRange = &d.sheets[0].compared_range;
    assert_eq!((range.start, range.end), (Some((1, 1)), Some((3, 2))));
    let addr: &CellAddress = &d.sheets[0].cell_diffs[0].address;
    assert_eq!((addr.row, addr.col, addr.a1.as_str()), (1, 1, "A1"));
}
