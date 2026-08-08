use std::process::ExitCode;

pub const SUCCESS_CODE: u8 = 0;
pub const FAILURE_CODE: u8 = 1;
pub const USAGE_CODE: u8 = 2;

pub fn success() -> ExitCode {
    ExitCode::from(SUCCESS_CODE)
}

pub fn failure() -> ExitCode {
    ExitCode::from(FAILURE_CODE)
}

pub fn usage() -> ExitCode {
    ExitCode::from(USAGE_CODE)
}

pub fn from_clap_code(code: i32) -> ExitCode {
    if code == i32::from(SUCCESS_CODE) {
        success()
    } else {
        usage()
    }
}

#[cfg(test)]
mod tests {
    use super::{FAILURE_CODE, SUCCESS_CODE, USAGE_CODE};

    #[test]
    fn exit_code_contract_is_stable() {
        assert_eq!(SUCCESS_CODE, 0);
        assert_eq!(FAILURE_CODE, 1);
        assert_eq!(USAGE_CODE, 2);
    }
}
