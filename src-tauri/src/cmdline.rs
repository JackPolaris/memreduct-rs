//! Command-line parsing (mirrors the original `-clean` / `-clean:full`).

use std::env;

/// Command-line actions understood by the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandLineAction {
    /// `-clean`: clean the default memory regions.
    CleanDefault,
    /// `-clean:full`: clean all memory regions.
    CleanFull,
    None,
}

/// Parse an iterator of argument strings for known actions.
pub fn parse_args<I, S>(args: I) -> CommandLineAction
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    for arg in args {
        let lower = arg.as_ref().to_lowercase();
        if lower == "-clean" || lower == "/clean" || lower == "--clean" {
            return CommandLineAction::CleanDefault;
        }
        if lower == "-clean:full" || lower == "/clean:full" || lower == "--clean:full" {
            return CommandLineAction::CleanFull;
        }
    }
    CommandLineAction::None
}

/// Parse the process command line for known actions.
pub fn parse() -> CommandLineAction {
    parse_args(env::args().skip(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_clean() {
        assert_eq!(parse_args(["-clean"]), CommandLineAction::CleanDefault);
        assert_eq!(parse_args(["-CLEAN"]), CommandLineAction::CleanDefault);
        assert_eq!(parse_args(["/clean"]), CommandLineAction::CleanDefault);
    }

    #[test]
    fn parses_clean_full() {
        assert_eq!(parse_args(["-clean:full"]), CommandLineAction::CleanFull);
        assert_eq!(parse_args(["/clean:full"]), CommandLineAction::CleanFull);
    }

    #[test]
    fn ignores_unrelated() {
        assert_eq!(parse_args(["/something", "foo"]), CommandLineAction::None);
        assert_eq!(parse_args([""]), CommandLineAction::None);
    }
}
