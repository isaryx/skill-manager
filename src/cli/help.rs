use clap::builder::StyledStr;
use clap::Command;

const GROUPED_HELP_TEMPLATE: &str = "\
{about-with-newline}
{usage-heading} {usage}

{before-help}
Options:
{options}
{after-help}\
";

const PROJECT_COMMANDS: &[&str] = &[
    "init",
    "use-profiles",
    "add-profile",
    "remove-profile",
    "sync",
    "status",
    "use-agents",
    "add-agent",
    "remove-agent",
    "destroy",
];

const STORE_COMMANDS: &[&str] = &[
    "import",
    "profile",
    "skill",
    "ls",
    "scan",
    "doctor",
];

/// Root help with gh-style `PROJECT COMMANDS` / `STORE COMMANDS` sections.
pub fn apply_grouped_help(mut cmd: Command) -> Command {
    let grouped = format_grouped_commands(&cmd);
    cmd = cmd
        .help_template(GROUPED_HELP_TEMPLATE)
        .before_help(grouped);
    cmd
}

fn format_grouped_commands(cmd: &Command) -> String {
    let width = PROJECT_COMMANDS
        .iter()
        .chain(STORE_COMMANDS.iter())
        .map(|name| name.len())
        .max()
        .unwrap_or(0)
        .max("help".len());
    let mut out = String::new();
    write_section(&mut out, "PROJECT COMMANDS", cmd, PROJECT_COMMANDS, width);
    out.push('\n');
    write_section(&mut out, "STORE COMMANDS", cmd, STORE_COMMANDS, width);
    out.push_str("\n  ");
    out.push_str("help");
    for _ in "help".len()..width {
        out.push(' ');
    }
    out.push_str("  Print this message or the help of the given subcommand(s)\n");
    out
}

fn write_section(out: &mut String, heading: &str, cmd: &Command, names: &[&str], width: usize) {
    out.push_str(heading);
    out.push('\n');
    for name in names {
        let Some(sub) = cmd.find_subcommand(name) else {
            continue;
        };
        let about = sub
            .get_about()
            .or_else(|| sub.get_long_about())
            .map(plain_about)
            .unwrap_or_default();
        out.push_str("  ");
        out.push_str(name);
        for _ in name.len()..width {
            out.push(' ');
        }
        if !about.is_empty() {
            out.push_str("  ");
            out.push_str(&about);
        }
        out.push('\n');
    }
}

fn plain_about(text: &StyledStr) -> String {
    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    use crate::Cli;

    #[test]
    fn grouped_help_lists_project_and_store_sections() {
        let cmd = apply_grouped_help(Cli::command());
        let grouped = format_grouped_commands(&cmd);
        assert!(grouped.contains("PROJECT COMMANDS"));
        assert!(grouped.contains("STORE COMMANDS"));
        assert!(grouped.contains("  init"));
        assert!(grouped.contains("  import"));
        assert!(grouped.contains("  add-profile"));
        assert!(grouped.contains("  remove-profile"));
        assert!(grouped.contains("  doctor"));
    }

    #[test]
    fn grouped_help_omits_flat_commands_section() {
        let rendered = apply_grouped_help(Cli::command())
            .render_help()
            .to_string();
        assert!(rendered.contains("PROJECT COMMANDS"));
        assert!(rendered.contains("STORE COMMANDS"));
        assert!(!rendered.contains("Commands:\n"));
    }
}
