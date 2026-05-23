use std::sync::{Arc, Mutex};

use tracing_subscriber::fmt::MakeWriter;

use super::AuroraFormatter;

// ── helper: capture log output into a String ──────────────────────────────────

#[derive(Clone)]
struct BufWriter(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for BufWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for BufWriter {
    type Writer = BufWriter;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

fn capture<F: FnOnce()>(f: F) -> String {
    let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
    let writer = BufWriter(buf.clone());
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(writer)
        .event_format(AuroraFormatter)
        .finish();
    tracing::subscriber::with_default(subscriber, f);
    let bytes = buf.lock().unwrap().clone();
    String::from_utf8(bytes).expect("log output must be UTF-8")
}

// ── end-to-end formatting tests ───────────────────────────────────────────────

#[test]
fn formats_message_and_structured_fields() {
    let output = capture(|| {
        tracing::info!(action = "integrations", elapsed_ms = 12u64, "tool call");
    });
    assert!(output.contains("tool call"), "message missing:\n{output}");
    assert!(
        output.contains("action=integrations"),
        "action field missing:\n{output}"
    );
    assert!(
        output.contains("elapsed_ms=12"),
        "elapsed_ms field missing:\n{output}"
    );
}

#[test]
fn formats_warn_level() {
    let output = capture(|| {
        tracing::warn!(error = "timeout", "upstream slow");
    });
    assert!(output.contains("WARN"), "WARN level missing:\n{output}");
    assert!(
        output.contains("upstream slow"),
        "message missing:\n{output}"
    );
    assert!(
        output.contains("error=timeout"),
        "error field missing:\n{output}"
    );
}

#[test]
fn formats_error_level() {
    let output = capture(|| {
        tracing::error!(status = 500u32, "request failed");
    });
    assert!(output.contains("ERROR"), "ERROR level missing:\n{output}");
    assert!(
        output.contains("request failed"),
        "message missing:\n{output}"
    );
}

#[test]
fn values_with_spaces_are_quoted() {
    let output = capture(|| {
        tracing::info!(error = "connection refused", "event");
    });
    assert!(
        output.contains(r#"error="connection refused""#),
        "space-containing value should be quoted:\n{output}"
    );
}

#[test]
fn no_ansi_codes_when_ansi_disabled() {
    let output = capture(|| {
        tracing::info!(key = "val", "msg");
    });
    assert!(
        !output.contains('\x1b'),
        "ANSI escape codes must not appear when ansi=false:\n{output}"
    );
}

// ── helper function unit tests ────────────────────────────────────────────────

#[test]
fn format_field_value_quotes_whitespace() {
    use super::format_field_value;
    assert_eq!(format_field_value("hello world"), r#""hello world""#);
    assert_eq!(format_field_value("nospace"), "nospace");
}

#[test]
fn should_skip_suppresses_false_flags() {
    use super::should_skip_field;
    assert!(should_skip_field("subject_scoped", "false"));
    assert!(should_skip_field("destructive", "false"));
    assert!(!should_skip_field("subject_scoped", "true"));
    assert!(!should_skip_field("error", "false"));
}

#[test]
fn sanitize_strips_c0_controls() {
    use super::sanitize_field_value;
    let injected = "tool\x1b[31mFAKE";
    let sanitized = sanitize_field_value(injected);
    assert!(!sanitized.contains('\x1b'), "ESC should be replaced");
    assert!(
        sanitized.contains('\u{FFFD}'),
        "should contain replacement char"
    );
}

#[test]
fn sanitize_preserves_tab_and_newline() {
    use super::sanitize_field_value;
    let value = "hello\tworld\nline2";
    assert_eq!(
        sanitize_field_value(value),
        value,
        "tab and newline must not be replaced"
    );
}

#[test]
fn sanitize_is_noop_for_clean_values() {
    use super::sanitize_field_value;
    let value = "upstream-name/tool.call";
    let sanitized = sanitize_field_value(value);
    assert!(
        matches!(sanitized, std::borrow::Cow::Borrowed(_)),
        "clean values should borrow (zero allocation)"
    );
}
