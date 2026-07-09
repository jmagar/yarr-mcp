use anyhow::Result;

use super::{assertions, matrix, process, report};

pub(super) fn run(
    report: &mut report::Report,
    yarr: &process::YarrProcess,
    matrix: &matrix::Matrix,
) -> Result<()> {
    for service in &matrix.services {
        let status = yarr.json(&[&service.name, "status"])?;
        assertions::assert_value(&status, &service.status)?;
        report.pass(
            format!("service_status {}", service.name),
            format!("semantic status matched ({})", service.kind),
        );

        for get_case in &service.get {
            let payload = yarr.json(&[&service.name, "get", "--path", &get_case.path])?;
            assertions::assert_value(&payload, &get_case.expectation)?;
            report.pass(
                format!("api_get {} {}", service.name, get_case.path),
                "semantic GET matched",
            );
        }

        assert_post_error(report, yarr, service)?;
    }
    Ok(())
}

fn assert_post_error(
    report: &mut report::Report,
    yarr: &process::YarrProcess,
    service: &matrix::ServiceCase,
) -> Result<()> {
    let body = service.post_expected_error.body.to_string();
    let args = vec![
        service.name.as_str(),
        "post",
        "--path",
        service.post_expected_error.path.as_str(),
        "--body",
        body.as_str(),
    ];

    let output = yarr.output(&args)?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assertions::assert_expected_error(&text, &service.post_expected_error.error_contains_any)?;

    report.pass(
        format!("api_post upstream error {}", service.name),
        "api_post reached upstream and returned the expected service error shape",
    );
    Ok(())
}
