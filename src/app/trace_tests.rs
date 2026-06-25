use super::{optional_query, paging_query};

#[test]
fn optional_query_omits_empty_values() {
    assert!(optional_query("period", None).is_empty());
    assert!(optional_query("period", Some("")).is_empty());
    assert_eq!(
        optional_query("period", Some("week")),
        vec![("period", "week".to_string())]
    );
}

#[test]
fn paging_query_uses_tracearr_page_size_wire_name() {
    assert_eq!(
        paging_query(Some(2), Some(50)),
        vec![("page", "2".to_string()), ("pageSize", "50".to_string())]
    );
}
