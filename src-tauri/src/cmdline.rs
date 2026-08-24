//! Command-line parsing (mirrors the original `-clean` / `-clean:full`, plus a
//! single-use `-clean-once <mask>` used by the elevation flow).

use std::env;

/// Command-line actions understood by the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandLineAction {
    /// `-clean`: clean the default memory regions.
    CleanDefault,
    /// `-clean:full`: clean all memory regions.
    CleanFull,
    /// `-clean-once <mask>`: perform one cleanup with the given mask and exit
    /// (used by the elevated helper process).
    CleanOnce(u32),
    None,
}

/// Parse an iterator of argument strings for known actions.
pub fn parse_args<I, S>(args: I) -> CommandLineAction
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args: Vec<String> = args.into_iter().map(|a| a.as_ref().to_string()).collect();
    let mut i = 0;
    while i < args.len() {
        let lower = args[i].to_lowercase();
        if lower == "-clean-once" || lower == "/clean-once" || lower == "--clean-once" {
            // Optional mask follows this argument.
            let mask = args
                .get(i + 1)
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(crate::memory::mask::ALL);
            return CommandLineAction::CleanOnce(mask);
        }
        if lower == "-clean" || lower == "/clean" || lower == "--clean" {
            return CommandLineAction::CleanDefault;
        }
        if lower == "-clean:full" || lower == "/clean:full" || lower == "--clean:full" {
            return CommandLineAction::CleanFull;
        }
        i += 1;
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

    #[test]
    fn parses_clean_once() {
        assert_eq!(
            parse_args(["-clean-once", "255"]),
            CommandLineAction::CleanOnce(255)
        );
        // Missing mask → full mask (all 8 regions = 0xFF).
        assert_eq!(
            parse_args(["-clean-once"]),
            CommandLineAction::CleanOnce(crate::memory::mask::ALL)
        );
    }
}
