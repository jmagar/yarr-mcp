use super::paging_query;

#[test]
fn paging_query_omits_absent_values_and_stringifies_present_values() {
    assert!(paging_query(None, None).is_empty());
    assert_eq!(
        paging_query(Some(10), Some(25)),
        vec![("start", "10".to_string()), ("length", "25".to_string())]
    );
}
