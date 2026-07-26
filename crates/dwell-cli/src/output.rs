//! Output formatting — colored terminal output and JSON mode.

use console::style;

pub struct Output {
    pub json_mode: bool,
    pub verbose: u8,
    pub quiet: bool,
}

impl Output {
    pub fn new(json_mode: bool, verbose: u8, quiet: bool) -> Self {
        Output {
            json_mode,
            verbose,
            quiet,
        }
    }

    pub fn info(&self, msg: &str) {
        if self.quiet {
            return;
        }
        if self.json_mode {
            println!(r#"{{"level":"info","message":"{}"}}"#, msg);
        } else {
            eprintln!("{} {}", style("→").cyan(), msg);
        }
    }

    pub fn success(&self, msg: &str) {
        if self.quiet {
            return;
        }
        if self.json_mode {
            println!(r#"{{"level":"success","message":"{}"}}"#, msg);
        } else {
            eprintln!("{} {}", style("✓").green(), msg);
        }
    }

    pub fn warn(&self, msg: &str) {
        if self.json_mode {
            println!(r#"{{"level":"warn","message":"{}"}}"#, msg);
        } else {
            eprintln!("{} {}", style("!").yellow(), msg);
        }
    }

    pub fn error(&self, msg: &str) {
        if self.json_mode {
            println!(r#"{{"level":"error","message":"{}"}}"#, msg);
        } else {
            eprintln!("{} {}", style("✗").red(), msg);
        }
    }

    pub fn apply_result(&self, result: &dwell_core::ApplyResult) {
        if self.json_mode {
            println!("{}", serde_json::to_string(result).unwrap_or_default());
            return;
        }

        let icon = match result.action {
            dwell_core::ApplyAction::Created => style("+").green(),
            dwell_core::ApplyAction::Updated => style("~").yellow(),
            dwell_core::ApplyAction::Deleted => style("-").red(),
            dwell_core::ApplyAction::Skipped => style("·").dim(),
            dwell_core::ApplyAction::Symlinked => style("@").cyan(),
        };

        if result.success {
            eprintln!(" {} {}", icon, result.path.display());
        } else {
            eprintln!(
                " {} {} — {}",
                style("✗").red(),
                result.path.display(),
                result.error.as_deref().unwrap_or("unknown error")
            );
        }
    }
}
