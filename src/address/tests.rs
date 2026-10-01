use super::*;

#[test]
fn col_label_single_letters() {
    assert_eq!(col_to_label(1), "A");
    assert_eq!(col_to_label(26), "Z");
}

#[test]
fn col_label_double_letters() {
    assert_eq!(col_to_label(27), "AA");
    assert_eq!(col_to_label(52), "AZ");
    assert_eq!(col_to_label(53), "BA");
    assert_eq!(col_to_label(702), "ZZ");
}

#[test]
fn col_label_triple_letters() {
    assert_eq!(col_to_label(703), "AAA");
    assert_eq!(col_to_label(16_384), MAX_COL_LABEL);
}

#[test]
fn cell_address_new_valid() {
    let addr = CellAddress::new(1, 1).unwrap();
    assert_eq!(addr.a1, "A1");
    assert_eq!(addr.row, 1);
    assert_eq!(addr.col, 1);

    let last = CellAddress::new(MAX_ROW, MAX_COL).unwrap();
    assert_eq!(last.a1, "XFD1048576");
}

#[test]
fn cell_address_new_out_of_bounds() {
    assert!(CellAddress::new(0, 1).is_none());
    assert!(CellAddress::new(1, 0).is_none());
    assert!(CellAddress::new(MAX_ROW + 1, 1).is_none());
    assert!(CellAddress::new(1, MAX_COL + 1).is_none());
}

#[test]
fn sort_order_is_row_col_not_a1_lexicographic() {
    let a2 = CellAddress::new(2, 1).unwrap();
    let a10 = CellAddress::new(10, 1).unwrap();
    assert!(a2 < a10, "A10 must sort after A2 (numeric row, not lex)");
}
