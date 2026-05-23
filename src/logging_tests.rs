use super::*;

#[test]
fn log_file_max_bytes_is_10mb() {
    assert_eq!(LOG_FILE_MAX_BYTES, 10 * 1024 * 1024);
}

#[test]
fn truncate_is_noop_when_file_absent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.log").to_path_buf();
    assert!(truncate_log_if_needed(&path).is_ok());
}

#[test]
fn truncate_is_noop_when_file_small() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.log").to_path_buf();
    std::fs::write(&path, b"small content").unwrap();
    truncate_log_if_needed(&path).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"small content");
}

#[test]
fn truncate_clears_large_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.log").to_path_buf();
    let big = vec![b'x'; (LOG_FILE_MAX_BYTES + 1) as usize];
    std::fs::write(&path, &big).unwrap();
    truncate_log_if_needed(&path).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"");
}
