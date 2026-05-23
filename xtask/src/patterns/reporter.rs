#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FindingLevel {
    Ok,
    Warn,
    Fail,
}

struct PatternFinding {
    level: FindingLevel,
    check: &'static str,
    message: String,
}

#[derive(Default)]
pub(super) struct PatternReporter {
    findings: Vec<PatternFinding>,
}

impl PatternReporter {
    pub(super) fn ok(&mut self, check: &'static str, message: impl Into<String>) {
        self.findings.push(PatternFinding {
            level: FindingLevel::Ok,
            check,
            message: message.into(),
        });
    }

    pub(super) fn warn(&mut self, check: &'static str, message: impl Into<String>) {
        self.findings.push(PatternFinding {
            level: FindingLevel::Warn,
            check,
            message: message.into(),
        });
    }

    pub(super) fn fail(&mut self, check: &'static str, message: impl Into<String>) {
        self.findings.push(PatternFinding {
            level: FindingLevel::Fail,
            check,
            message: message.into(),
        });
    }

    pub(super) fn print(&self) {
        for finding in &self.findings {
            println!(
                "{}: {}: {}",
                finding.level.as_str(),
                finding.check,
                finding.message
            );
        }
    }

    pub(super) fn print_json(&self) {
        println!("{{");
        println!("  \"findings\": [");
        for (index, finding) in self.findings.iter().enumerate() {
            let comma = if index + 1 == self.findings.len() {
                ""
            } else {
                ","
            };
            println!(
                "    {{\"level\":\"{}\",\"check\":\"{}\",\"message\":\"{}\"}}{}",
                finding.level.as_str(),
                json_escape(finding.check),
                json_escape(&finding.message),
                comma
            );
        }
        println!("  ],");
        println!("  \"has_failures\": {},", self.has_failures());
        println!("  \"has_warnings\": {}", self.has_warnings());
        println!("}}");
    }

    pub(super) fn has_failures(&self) -> bool {
        self.findings
            .iter()
            .any(|finding| finding.level == FindingLevel::Fail)
    }

    pub(super) fn has_warnings(&self) -> bool {
        self.findings
            .iter()
            .any(|finding| finding.level == FindingLevel::Warn)
    }
}

impl FindingLevel {
    fn as_str(self) -> &'static str {
        match self {
            FindingLevel::Ok => "OK",
            FindingLevel::Warn => "WARN",
            FindingLevel::Fail => "FAIL",
        }
    }
}

fn json_escape(value: &str) -> String {
    value
        .chars()
        .flat_map(|ch| match ch {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '"' => "\\\"".chars().collect(),
            '\n' => "\\n".chars().collect(),
            '\r' => "\\r".chars().collect(),
            '\t' => "\\t".chars().collect(),
            other => vec![other],
        })
        .collect()
}
