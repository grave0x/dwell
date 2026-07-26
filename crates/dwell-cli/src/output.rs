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

    /// Section header — bright cyan, bold
    pub fn section(&self, msg: &str) {
        if self.quiet {
            return;
        }
        if self.json_mode {
            println!(r#"{{"level":"section","message":"{}"}}"#, msg);
        } else {
            eprintln!("{}", style(format!("══ {} ══", msg)).cyan().bright());
        }
    }

    /// Info — blue arrow, dim message
    pub fn info(&self, msg: &str) {
        if self.quiet {
            return;
        }
        if self.json_mode {
            println!(r#"{{"level":"info","message":"{}"}}"#, msg);
        } else {
            eprintln!(" {} {}", style("→").cyan(), style(msg).dim());
        }
    }

    /// Success — green checkmark, green message
    pub fn success(&self, msg: &str) {
        if self.quiet {
            return;
        }
        if self.json_mode {
            println!(r#"{{"level":"success","message":"{}"}}"#, msg);
        } else {
            eprintln!(" {} {}", style("✓").green().bright(), style(msg).green());
        }
    }

    /// Warning — yellow exclamation, yellow message
    pub fn warn(&self, msg: &str) {
        if self.json_mode {
            println!(r#"{{"level":"warn","message":"{}"}}"#, msg);
        } else {
            eprintln!(" {} {}", style("!").yellow().bright(), style(msg).yellow());
        }
    }

    /// Error — red X, red message
    pub fn error(&self, msg: &str) {
        if self.json_mode {
            println!(r#"{{"level":"error","message":"{}"}}"#, msg);
        } else {
            eprintln!(" {} {}", style("✗").red().bright(), style(msg).red().bright());
        }
    }

    /// Step indicator — bold magenta for phase numbers
    pub fn step(&self, phase: &str, msg: &str) {
        if self.quiet {
            return;
        }
        if self.json_mode {
            println!(r#"{{"level":"step","phase":"{}","message":"{}"}}"#, phase, msg);
        } else {
            eprintln!(" {} {}",
                style(phase).magenta().bright(),
                style(msg).bold(),
            );
        }
    }

    /// Apply result — colored per action type
    pub fn apply_result(&self, result: &dwell_core::ApplyResult) {
        if self.json_mode {
            println!("{}", serde_json::to_string(result).unwrap_or_default());
            return;
        }

        let icon = match result.action {
            dwell_core::ApplyAction::Created => style("+").green(),
            dwell_core::ApplyAction::Updated => style("~").yellow(),
            dwell_core::ApplyAction::Deleted => style("-").red().bright(),
            dwell_core::ApplyAction::Skipped => style("·").dim(),
            dwell_core::ApplyAction::Symlinked => style("@").cyan(),
        };

        let path = style(result.path.display()).bold();
        if result.success {
            eprintln!(" {} {}", icon, path);
        } else {
            let err = result.error.as_deref().unwrap_or("unknown error");
            eprintln!(" {} {} — {}",
                style("✗").red().bright(),
                path,
                style(err).red().dim(),
            );
        }
    }

    /// Title bar — prominent header with surrounding lines
    pub fn title(&self, msg: &str) {
        if self.quiet {
            return;
        }
        if self.json_mode {
            println!(r#"{{"level":"title","message":"{}"}}"#, msg);
        } else {
            let line = style("━".repeat(60)).dim();
            eprintln!("\n{}", line);
            eprintln!("  {}", style(msg).bold().cyan().bright());
            eprintln!("{}\n", line);
        }
    }

    /// Show a colored key-value pair
    pub fn kv(&self, key: &str, value: &str) {
        if self.quiet {
            return;
        }
        if self.json_mode {
            println!(r#"{{"level":"kv","key":"{}","value":"{}"}}"#, key, value);
        } else {
            eprintln!("  {} {}",
                style(key).cyan().dim(),
                style(value).bold(),
            );
        }
    }

    /// File path — underlined for emphasis
    pub fn path(&self, path: &str) {
        if self.quiet {
            return;
        }
        if self.json_mode {
            println!(r#"{{"level":"path","path":"{}"}}"#, path);
        } else {
            eprintln!("    {}", style(path).underlined().dim());
        }
    }
}
