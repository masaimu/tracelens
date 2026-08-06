use std::collections::BTreeMap;

pub const TRACE_ID_LEN: usize = 32;
pub const SPAN_ID_LEN: usize = 16;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalSpan {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub service_name: String,
    pub name: String,
    pub kind: Option<i64>,
    pub start_ns: u64,
    pub end_ns: u64,
    pub status_code: Option<i64>,
    pub attributes: BTreeMap<String, String>,
}

impl CanonicalSpan {
    pub fn duration_ns(&self) -> u64 {
        self.end_ns.saturating_sub(self.start_ns)
    }

    pub fn is_error(&self) -> bool {
        self.status_code == Some(2)
            || self
                .attributes
                .get("http.status_code")
                .and_then(|value| value.parse::<u16>().ok())
                .is_some_and(|status| status >= 500)
            || self
                .attributes
                .get("rpc.grpc.status_code")
                .is_some_and(|value| value != "0")
    }
}

pub fn normalize_hex_id(value: &str, expected_len: usize) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.len() != expected_len {
        return Err(format!(
            "expected {expected_len} hex characters, got {}",
            trimmed.len()
        ));
    }

    if !trimmed.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("expected only hexadecimal characters".to_string());
    }

    Ok(trimmed.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::{SPAN_ID_LEN, TRACE_ID_LEN, normalize_hex_id};

    #[test]
    fn normalizes_uppercase_hex_ids() {
        let trace_id = normalize_hex_id("5B8EFFF798038103D269B633813FC60C", TRACE_ID_LEN)
            .expect("trace id should normalize");
        let span_id =
            normalize_hex_id("EEE19B7EC3C1B174", SPAN_ID_LEN).expect("span id should normalize");

        assert_eq!(trace_id, "5b8efff798038103d269b633813fc60c");
        assert_eq!(span_id, "eee19b7ec3c1b174");
    }

    #[test]
    fn rejects_wrong_length_ids() {
        let error = normalize_hex_id("abc", TRACE_ID_LEN).expect_err("id should be invalid");

        assert!(error.contains("expected 32 hex characters"));
    }
}
