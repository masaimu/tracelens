use std::env;
use std::fmt::Display;
use std::io::{self, IsTerminal};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextStyle {
    enabled: bool,
}

impl TextStyle {
    pub fn from_mode(mode: ColorMode) -> Self {
        let enabled = match mode {
            ColorMode::Auto => io::stdout().is_terminal() && env::var_os("NO_COLOR").is_none(),
            ColorMode::Always => true,
            ColorMode::Never => false,
        };
        Self { enabled }
    }

    pub fn section<T: Display>(&self, value: T) -> String {
        self.paint("1;36", value)
    }

    pub fn table_header<T: Display>(&self, value: T) -> String {
        self.paint("1", value)
    }

    pub fn ok<T: Display>(&self, value: T) -> String {
        self.paint("32", value)
    }

    pub fn error<T: Display>(&self, value: T) -> String {
        self.paint("31", value)
    }

    pub fn warning<T: Display>(&self, value: T) -> String {
        self.paint("33", value)
    }

    pub fn duration<T: Display>(&self, value: T) -> String {
        self.paint("1;33", value)
    }

    pub fn identifier<T: Display>(&self, value: T) -> String {
        self.paint("2", value)
    }

    pub fn service<T: Display>(&self, value: T) -> String {
        self.paint("34", value)
    }

    pub fn critical<T: Display>(&self, value: T) -> String {
        self.paint("1;35", value)
    }

    pub fn concurrent<T: Display>(&self, value: T) -> String {
        self.paint("36", value)
    }

    pub fn muted<T: Display>(&self, value: T) -> String {
        self.paint("2", value)
    }

    fn paint<T: Display>(&self, code: &'static str, value: T) -> String {
        let value = value.to_string();
        if self.enabled {
            format!("\x1b[{code}m{value}\x1b[0m")
        } else {
            value
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::output::style::{ColorMode, TextStyle};

    #[test]
    fn always_emits_ansi_escape() {
        let style = TextStyle::from_mode(ColorMode::Always);

        assert_eq!(style.error("failed"), "\x1b[31mfailed\x1b[0m");
    }

    #[test]
    fn never_omits_ansi_escape() {
        let style = TextStyle::from_mode(ColorMode::Never);

        assert_eq!(style.error("failed"), "failed");
    }
}
