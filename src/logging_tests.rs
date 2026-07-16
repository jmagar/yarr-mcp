use super::*;

#[test]
fn log_file_max_bytes_is_10mb() {
    assert_eq!(LOG_FILE_MAX_BYTES, 10 * 1024 * 1024);
}

#[test]
fn rotating_file_preserves_small_existing_log() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.log").to_path_buf();
    std::fs::write(&path, b"small content").unwrap();
    let mut writer = RotatingFile::open(path.clone()).unwrap();
    writer.append(b" plus").unwrap();
    writer.file.as_mut().unwrap().flush().unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"small content plus");
}

#[test]
fn rotating_file_rotates_while_process_is_running() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.log").to_path_buf();
    std::fs::write(&path, vec![b'x'; (LOG_FILE_MAX_BYTES - 2) as usize]).unwrap();
    let mut writer = RotatingFile::open(path.clone()).unwrap();
    writer.append(b"four").unwrap();
    writer.file.as_mut().unwrap().flush().unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"four");
    assert_eq!(
        std::fs::metadata(backup_path(&path, 1)).unwrap().len(),
        LOG_FILE_MAX_BYTES - 2
    );
}

#[test]
fn non_blocking_writer_enqueues_one_complete_event() {
    let (sender, receiver) = sync_channel(1);
    let mut writer = NonBlockingLogWriter {
        sender,
        buffer: Vec::new(),
    };
    writer.write_all(b"{\"message\":").unwrap();
    writer.write_all(b"\"ok\"}\n").unwrap();
    drop(writer);
    assert_eq!(receiver.recv().unwrap(), b"{\"message\":\"ok\"}\n");
}

#[test]
fn oversized_existing_log_is_rotated_to_bounded_backup() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.log");
    std::fs::write(&path, vec![b'x'; LOG_FILE_MAX_BYTES as usize + 1]).unwrap();
    let writer = RotatingFile::open(path.clone()).unwrap();
    assert_eq!(writer.bytes, 0);
    assert_eq!(
        std::fs::metadata(backup_path(&path, 1)).unwrap().len(),
        LOG_FILE_MAX_BYTES
    );
}
