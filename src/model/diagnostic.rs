use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Severity {
    Warning,
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Warning => formatter.write_str("warning"),
            Severity::Error => formatter.write_str("error"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiagnosticScope {
    File,
    Trace,
    Span,
}

impl fmt::Display for DiagnosticScope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiagnosticScope::File => formatter.write_str("file"),
            DiagnosticScope::Trace => formatter.write_str("trace"),
            DiagnosticScope::Span => formatter.write_str("span"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub scope: DiagnosticScope,
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub location: Option<String>,
}

impl Diagnostic {
    pub fn warning(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            scope: DiagnosticScope::File,
            severity: Severity::Warning,
            code,
            message: message.into(),
            trace_id: None,
            span_id: None,
            location: None,
        }
    }

    pub fn error(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            scope: DiagnosticScope::File,
            severity: Severity::Error,
            code,
            message: message.into(),
            trace_id: None,
            span_id: None,
            location: None,
        }
    }

    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        if self.scope == DiagnosticScope::File {
            self.scope = DiagnosticScope::Trace;
        }
        self.trace_id = Some(trace_id.into());
        self
    }

    pub fn with_span_id(mut self, span_id: impl Into<String>) -> Self {
        self.scope = DiagnosticScope::Span;
        self.span_id = Some(span_id.into());
        self
    }

    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }
}
