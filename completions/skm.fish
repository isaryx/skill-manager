# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_skm_global_optspecs
    string join \n v/verbose store= json dry-run color= h/help V/version
end

function __fish_skm_needs_command
    # Figure out if the current invocation already has a command.
    set -l cmd (commandline -opc)
    set -e cmd[1]
    argparse -s (__fish_skm_global_optspecs) -- $cmd 2>/dev/null
    or return
    if set -q argv[1]
        # Also print the command, so this can be used to figure out what it is.
        echo $argv[1]
        return 1
    end
    return 0
end

function __fish_skm_using_subcommand
    set -l cmd (__fish_skm_needs_command)
    test -z "$cmd"
    and return 1
    contains -- $cmd[1] $argv
end

complete -c skm -n "__fish_skm_needs_command" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_needs_command" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_needs_command" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_needs_command" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `skill ls`, `doctor` only)'
complete -c skm -n "__fish_skm_needs_command" -l dry-run -d 'Preview changes without writing (`sync`, `use-profile`, `skill rm` only)'
complete -c skm -n "__fish_skm_needs_command" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c skm -n "__fish_skm_needs_command" -s V -l version -d 'Print version'
complete -c skm -n "__fish_skm_needs_command" -f -a "init" -d 'Set up the skill store and project config (`.skm.toml`)'
complete -c skm -n "__fish_skm_needs_command" -f -a "import" -d 'Import a skill directory into the store'
complete -c skm -n "__fish_skm_needs_command" -f -a "profile" -d 'Create and manage profiles'
complete -c skm -n "__fish_skm_needs_command" -f -a "skill" -d 'Manage skills in the store'
complete -c skm -n "__fish_skm_needs_command" -f -a "use-profile" -d 'Activate a profile and sync skill links'
complete -c skm -n "__fish_skm_needs_command" -f -a "setup-agents" -d 'Choose which agents your config places skills into'
complete -c skm -n "__fish_skm_needs_command" -f -a "destroy" -d 'Tear down this project\'s skm setup (`.skm.toml` and store-owned links)'
complete -c skm -n "__fish_skm_needs_command" -f -a "sync" -d 'Refresh skill links without changing the active profile'
complete -c skm -n "__fish_skm_needs_command" -f -a "status" -d 'Show agent, active profile, linked skills, and name conflicts'
complete -c skm -n "__fish_skm_needs_command" -f -a "ls" -d 'List skills and profiles in the store'
complete -c skm -n "__fish_skm_needs_command" -f -a "scan" -d 'Refresh the on-disk skill index'
complete -c skm -n "__fish_skm_needs_command" -f -a "doctor" -d 'Read-only health report for the store, profiles, and skill links'
complete -c skm -n "__fish_skm_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c skm -n "__fish_skm_using_subcommand init" -l agent -d 'Target agents for this project (repeatable, comma-separated)' -r -f -a "claude-code\t'Claude Code (.claude/skills)'
cursor\t'Cursor (.cursor/skills)'
generic\t'Agent Skills (.agents/skills); Codex, Cursor, Gemini CLI, Copilot CLI'
gemini-cli\t'Gemini CLI (.gemini/skills)'
copilot-cli\t'Copilot CLI (.github/skills; ~/.copilot/skills with --user)'"
complete -c skm -n "__fish_skm_using_subcommand init" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand init" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand init" -l force -d 'Overwrite an existing `.skm.toml`'
complete -c skm -n "__fish_skm_using_subcommand init" -l accept-existing-skills -d 'Proceed when the agent skills directory already has entries (non-interactive)'
complete -c skm -n "__fish_skm_using_subcommand init" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand init" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `skill ls`, `doctor` only)'
complete -c skm -n "__fish_skm_using_subcommand init" -l dry-run -d 'Preview changes without writing (`sync`, `use-profile`, `skill rm` only)'
complete -c skm -n "__fish_skm_using_subcommand init" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c skm -n "__fish_skm_using_subcommand import" -l as -d 'Name to use in the store' -r
complete -c skm -n "__fish_skm_using_subcommand import" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand import" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand import" -l copy -d 'Copy the skill into the store (keeps the original)'
complete -c skm -n "__fish_skm_using_subcommand import" -l move -d 'Move the skill into the store (removes the original)'
complete -c skm -n "__fish_skm_using_subcommand import" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand import" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `skill ls`, `doctor` only)'
complete -c skm -n "__fish_skm_using_subcommand import" -l dry-run -d 'Preview changes without writing (`sync`, `use-profile`, `skill rm` only)'
complete -c skm -n "__fish_skm_using_subcommand import" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand profile; and not __fish_seen_subcommand_from setup extend ls show rm help" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand profile; and not __fish_seen_subcommand_from setup extend ls show rm help" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand profile; and not __fish_seen_subcommand_from setup extend ls show rm help" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand profile; and not __fish_seen_subcommand_from setup extend ls show rm help" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `skill ls`, `doctor` only)'
complete -c skm -n "__fish_skm_using_subcommand profile; and not __fish_seen_subcommand_from setup extend ls show rm help" -l dry-run -d 'Preview changes without writing (`sync`, `use-profile`, `skill rm` only)'
complete -c skm -n "__fish_skm_using_subcommand profile; and not __fish_seen_subcommand_from setup extend ls show rm help" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand profile; and not __fish_seen_subcommand_from setup extend ls show rm help" -f -a "setup" -d 'Choose skills for a profile (interactive; creates the profile if missing)'
complete -c skm -n "__fish_skm_using_subcommand profile; and not __fish_seen_subcommand_from setup extend ls show rm help" -f -a "extend" -d 'Choose which profiles this one inherits skills from (interactive; creates the profile if missing)'
complete -c skm -n "__fish_skm_using_subcommand profile; and not __fish_seen_subcommand_from setup extend ls show rm help" -f -a "ls" -d 'List profile names'
complete -c skm -n "__fish_skm_using_subcommand profile; and not __fish_seen_subcommand_from setup extend ls show rm help" -f -a "show" -d 'Show skills in a profile'
complete -c skm -n "__fish_skm_using_subcommand profile; and not __fish_seen_subcommand_from setup extend ls show rm help" -f -a "rm" -d 'Remove a profile'
complete -c skm -n "__fish_skm_using_subcommand profile; and not __fish_seen_subcommand_from setup extend ls show rm help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from setup" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from setup" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from setup" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from setup" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `skill ls`, `doctor` only)'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from setup" -l dry-run -d 'Preview changes without writing (`sync`, `use-profile`, `skill rm` only)'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from setup" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from extend" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from extend" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from extend" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from extend" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `skill ls`, `doctor` only)'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from extend" -l dry-run -d 'Preview changes without writing (`sync`, `use-profile`, `skill rm` only)'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from extend" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from ls" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from ls" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from ls" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from ls" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `skill ls`, `doctor` only)'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from ls" -l dry-run -d 'Preview changes without writing (`sync`, `use-profile`, `skill rm` only)'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from ls" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from show" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from show" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from show" -l tree -d 'Print the extend graph as a tree instead of a flat skill list'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from show" -s u -l user -d 'Use user-level config when checking which profile is active'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from show" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from show" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `skill ls`, `doctor` only)'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from show" -l dry-run -d 'Preview changes without writing (`sync`, `use-profile`, `skill rm` only)'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from show" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from rm" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from rm" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from rm" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from rm" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `skill ls`, `doctor` only)'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from rm" -l dry-run -d 'Preview changes without writing (`sync`, `use-profile`, `skill rm` only)'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from rm" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from help" -f -a "setup" -d 'Choose skills for a profile (interactive; creates the profile if missing)'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from help" -f -a "extend" -d 'Choose which profiles this one inherits skills from (interactive; creates the profile if missing)'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from help" -f -a "ls" -d 'List profile names'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from help" -f -a "show" -d 'Show skills in a profile'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from help" -f -a "rm" -d 'Remove a profile'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c skm -n "__fish_skm_using_subcommand skill; and not __fish_seen_subcommand_from ls setup rm help" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand skill; and not __fish_seen_subcommand_from ls setup rm help" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand skill; and not __fish_seen_subcommand_from ls setup rm help" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand skill; and not __fish_seen_subcommand_from ls setup rm help" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `skill ls`, `doctor` only)'
complete -c skm -n "__fish_skm_using_subcommand skill; and not __fish_seen_subcommand_from ls setup rm help" -l dry-run -d 'Preview changes without writing (`sync`, `use-profile`, `skill rm` only)'
complete -c skm -n "__fish_skm_using_subcommand skill; and not __fish_seen_subcommand_from ls setup rm help" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand skill; and not __fish_seen_subcommand_from ls setup rm help" -f -a "ls" -d 'List enabled skills in the store'
complete -c skm -n "__fish_skm_using_subcommand skill; and not __fish_seen_subcommand_from ls setup rm help" -f -a "setup" -d 'Choose which store skills are enabled (interactive; all enabled by default)'
complete -c skm -n "__fish_skm_using_subcommand skill; and not __fish_seen_subcommand_from ls setup rm help" -f -a "rm" -d 'Permanently remove a skill from the store'
complete -c skm -n "__fish_skm_using_subcommand skill; and not __fish_seen_subcommand_from ls setup rm help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from ls" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from ls" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from ls" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from ls" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `skill ls`, `doctor` only)'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from ls" -l dry-run -d 'Preview changes without writing (`sync`, `use-profile`, `skill rm` only)'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from ls" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from setup" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from setup" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from setup" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from setup" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `skill ls`, `doctor` only)'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from setup" -l dry-run -d 'Preview changes without writing (`sync`, `use-profile`, `skill rm` only)'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from setup" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from rm" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from rm" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from rm" -l force -d 'Remove without confirmation (required when stdin is not a TTY)'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from rm" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from rm" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `skill ls`, `doctor` only)'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from rm" -l dry-run -d 'Preview changes without writing (`sync`, `use-profile`, `skill rm` only)'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from rm" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from help" -f -a "ls" -d 'List enabled skills in the store'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from help" -f -a "setup" -d 'Choose which store skills are enabled (interactive; all enabled by default)'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from help" -f -a "rm" -d 'Permanently remove a skill from the store'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c skm -n "__fish_skm_using_subcommand use-profile" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand use-profile" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand use-profile" -s u -l user -d 'Use `~/.skm.toml` even when `./.skm.toml` exists'
complete -c skm -n "__fish_skm_using_subcommand use-profile" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand use-profile" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `skill ls`, `doctor` only)'
complete -c skm -n "__fish_skm_using_subcommand use-profile" -l dry-run -d 'Preview changes without writing (`sync`, `use-profile`, `skill rm` only)'
complete -c skm -n "__fish_skm_using_subcommand use-profile" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c skm -n "__fish_skm_using_subcommand setup-agents" -l agent -d 'Target agents (repeatable, comma-separated); omit to pick interactively' -r -f -a "claude-code\t'Claude Code (.claude/skills)'
cursor\t'Cursor (.cursor/skills)'
generic\t'Agent Skills (.agents/skills); Codex, Cursor, Gemini CLI, Copilot CLI'
gemini-cli\t'Gemini CLI (.gemini/skills)'
copilot-cli\t'Copilot CLI (.github/skills; ~/.copilot/skills with --user)'"
complete -c skm -n "__fish_skm_using_subcommand setup-agents" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand setup-agents" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand setup-agents" -s u -l user -d 'Use `~/.skm.toml` even when `./.skm.toml` exists'
complete -c skm -n "__fish_skm_using_subcommand setup-agents" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand setup-agents" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `skill ls`, `doctor` only)'
complete -c skm -n "__fish_skm_using_subcommand setup-agents" -l dry-run -d 'Preview changes without writing (`sync`, `use-profile`, `skill rm` only)'
complete -c skm -n "__fish_skm_using_subcommand setup-agents" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c skm -n "__fish_skm_using_subcommand destroy" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand destroy" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand destroy" -l force -d 'Destroy without confirmation (required when stdin is not a TTY)'
complete -c skm -n "__fish_skm_using_subcommand destroy" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand destroy" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `skill ls`, `doctor` only)'
complete -c skm -n "__fish_skm_using_subcommand destroy" -l dry-run -d 'Preview changes without writing (`sync`, `use-profile`, `skill rm` only)'
complete -c skm -n "__fish_skm_using_subcommand destroy" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c skm -n "__fish_skm_using_subcommand sync" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand sync" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand sync" -s u -l user -d 'Use `~/.skm.toml` even when `./.skm.toml` exists'
complete -c skm -n "__fish_skm_using_subcommand sync" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand sync" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `skill ls`, `doctor` only)'
complete -c skm -n "__fish_skm_using_subcommand sync" -l dry-run -d 'Preview changes without writing (`sync`, `use-profile`, `skill rm` only)'
complete -c skm -n "__fish_skm_using_subcommand sync" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand status" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand status" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand status" -s u -l user -d 'Use `~/.skm.toml` even when `./.skm.toml` exists'
complete -c skm -n "__fish_skm_using_subcommand status" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand status" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `skill ls`, `doctor` only)'
complete -c skm -n "__fish_skm_using_subcommand status" -l dry-run -d 'Preview changes without writing (`sync`, `use-profile`, `skill rm` only)'
complete -c skm -n "__fish_skm_using_subcommand status" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c skm -n "__fish_skm_using_subcommand ls" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand ls" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand ls" -s p -l profile -d 'List profiles only (same as `skm profile ls`)'
complete -c skm -n "__fish_skm_using_subcommand ls" -s s -l skill -d 'List skills only (same as `skm skill ls`)'
complete -c skm -n "__fish_skm_using_subcommand ls" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand ls" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `skill ls`, `doctor` only)'
complete -c skm -n "__fish_skm_using_subcommand ls" -l dry-run -d 'Preview changes without writing (`sync`, `use-profile`, `skill rm` only)'
complete -c skm -n "__fish_skm_using_subcommand ls" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand scan" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand scan" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand scan" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand scan" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `skill ls`, `doctor` only)'
complete -c skm -n "__fish_skm_using_subcommand scan" -l dry-run -d 'Preview changes without writing (`sync`, `use-profile`, `skill rm` only)'
complete -c skm -n "__fish_skm_using_subcommand scan" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand doctor" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand doctor" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand doctor" -s u -l user -d 'Use `~/.skm.toml` even when `./.skm.toml` exists'
complete -c skm -n "__fish_skm_using_subcommand doctor" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand doctor" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `skill ls`, `doctor` only)'
complete -c skm -n "__fish_skm_using_subcommand doctor" -l dry-run -d 'Preview changes without writing (`sync`, `use-profile`, `skill rm` only)'
complete -c skm -n "__fish_skm_using_subcommand doctor" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profile setup-agents destroy sync status ls scan doctor help" -f -a "init" -d 'Set up the skill store and project config (`.skm.toml`)'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profile setup-agents destroy sync status ls scan doctor help" -f -a "import" -d 'Import a skill directory into the store'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profile setup-agents destroy sync status ls scan doctor help" -f -a "profile" -d 'Create and manage profiles'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profile setup-agents destroy sync status ls scan doctor help" -f -a "skill" -d 'Manage skills in the store'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profile setup-agents destroy sync status ls scan doctor help" -f -a "use-profile" -d 'Activate a profile and sync skill links'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profile setup-agents destroy sync status ls scan doctor help" -f -a "setup-agents" -d 'Choose which agents your config places skills into'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profile setup-agents destroy sync status ls scan doctor help" -f -a "destroy" -d 'Tear down this project\'s skm setup (`.skm.toml` and store-owned links)'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profile setup-agents destroy sync status ls scan doctor help" -f -a "sync" -d 'Refresh skill links without changing the active profile'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profile setup-agents destroy sync status ls scan doctor help" -f -a "status" -d 'Show agent, active profile, linked skills, and name conflicts'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profile setup-agents destroy sync status ls scan doctor help" -f -a "ls" -d 'List skills and profiles in the store'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profile setup-agents destroy sync status ls scan doctor help" -f -a "scan" -d 'Refresh the on-disk skill index'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profile setup-agents destroy sync status ls scan doctor help" -f -a "doctor" -d 'Read-only health report for the store, profiles, and skill links'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profile setup-agents destroy sync status ls scan doctor help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c skm -n "__fish_skm_using_subcommand help; and __fish_seen_subcommand_from profile" -f -a "setup" -d 'Choose skills for a profile (interactive; creates the profile if missing)'
complete -c skm -n "__fish_skm_using_subcommand help; and __fish_seen_subcommand_from profile" -f -a "extend" -d 'Choose which profiles this one inherits skills from (interactive; creates the profile if missing)'
complete -c skm -n "__fish_skm_using_subcommand help; and __fish_seen_subcommand_from profile" -f -a "ls" -d 'List profile names'
complete -c skm -n "__fish_skm_using_subcommand help; and __fish_seen_subcommand_from profile" -f -a "show" -d 'Show skills in a profile'
complete -c skm -n "__fish_skm_using_subcommand help; and __fish_seen_subcommand_from profile" -f -a "rm" -d 'Remove a profile'
complete -c skm -n "__fish_skm_using_subcommand help; and __fish_seen_subcommand_from skill" -f -a "ls" -d 'List enabled skills in the store'
complete -c skm -n "__fish_skm_using_subcommand help; and __fish_seen_subcommand_from skill" -f -a "setup" -d 'Choose which store skills are enabled (interactive; all enabled by default)'
complete -c skm -n "__fish_skm_using_subcommand help; and __fish_seen_subcommand_from skill" -f -a "rm" -d 'Permanently remove a skill from the store'
