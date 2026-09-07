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
complete -c skm -n "__fish_skm_needs_command" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_needs_command" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_needs_command" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c skm -n "__fish_skm_needs_command" -s V -l version -d 'Print version'
complete -c skm -n "__fish_skm_needs_command" -f -a "init" -d 'Set up the skill store and project config (`.skm.toml`)'
complete -c skm -n "__fish_skm_needs_command" -f -a "import" -d 'Import a skill directory into the store (or `github:owner/repo` to register a remote)'
complete -c skm -n "__fish_skm_needs_command" -f -a "profile" -d 'Create and manage profiles'
complete -c skm -n "__fish_skm_needs_command" -f -a "skill" -d 'Manage skills in the store'
complete -c skm -n "__fish_skm_needs_command" -f -a "use-profiles" -d 'Choose active profiles and sync skill links (interactive)'
complete -c skm -n "__fish_skm_needs_command" -f -a "add-profile" -d 'Add a profile to the active set and sync skill links'
complete -c skm -n "__fish_skm_needs_command" -f -a "remove-profile" -d 'Remove a profile from the active set and sync skill links'
complete -c skm -n "__fish_skm_needs_command" -f -a "use-agents" -d 'Choose target agents interactively'
complete -c skm -n "__fish_skm_needs_command" -f -a "add-agent" -d 'Add a target agent to this setup'
complete -c skm -n "__fish_skm_needs_command" -f -a "remove-agent" -d 'Remove a target agent from this setup'
complete -c skm -n "__fish_skm_needs_command" -f -a "destroy" -d 'Tear down this project\'s skm setup (`.skm.toml` and store-owned links)'
complete -c skm -n "__fish_skm_needs_command" -f -a "sync" -d 'Refresh skill links without changing the active profiles'
complete -c skm -n "__fish_skm_needs_command" -f -a "status" -d 'Show target agents, active profiles, linked skills, and name conflicts'
complete -c skm -n "__fish_skm_needs_command" -f -a "ls" -d 'List skills and profiles in the store'
complete -c skm -n "__fish_skm_needs_command" -f -a "scan" -d 'Refresh the on-disk skill index'
complete -c skm -n "__fish_skm_needs_command" -f -a "search" -d 'Search indexed skill IDs and descriptions'
complete -c skm -n "__fish_skm_needs_command" -f -a "doctor" -d 'Read-only health report for the store, profiles, and skill links'
complete -c skm -n "__fish_skm_needs_command" -f -a "repo" -d 'Manage remote skill repositories (GitHub, GitLab, and other git hosts)'
complete -c skm -n "__fish_skm_needs_command" -f -a "update" -d 'Pull registered remote repositories (no agent symlink changes)'
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
complete -c skm -n "__fish_skm_using_subcommand init" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand init" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand init" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c skm -n "__fish_skm_using_subcommand import" -l as -d 'Name to use in the store' -r
complete -c skm -n "__fish_skm_using_subcommand import" -l repo -d 'Register under a remote repo name (e.g. `agent-skills/deploy`)' -r
complete -c skm -n "__fish_skm_using_subcommand import" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand import" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand import" -l copy -d 'Copy the skill into the store (keeps the original)'
complete -c skm -n "__fish_skm_using_subcommand import" -l move -d 'Move the skill into the store (removes the original)'
complete -c skm -n "__fish_skm_using_subcommand import" -l strict -d 'Fail when SKILL.md frontmatter is invalid (default: warn)'
complete -c skm -n "__fish_skm_using_subcommand import" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand import" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand import" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand import" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand profile; and not __fish_seen_subcommand_from setup extend ls show rm help" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand profile; and not __fish_seen_subcommand_from setup extend ls show rm help" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand profile; and not __fish_seen_subcommand_from setup extend ls show rm help" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand profile; and not __fish_seen_subcommand_from setup extend ls show rm help" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand profile; and not __fish_seen_subcommand_from setup extend ls show rm help" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
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
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from setup" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from setup" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from setup" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from extend" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from extend" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from extend" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from extend" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from extend" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from extend" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from ls" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from ls" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from ls" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from ls" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from ls" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from ls" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from show" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from show" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from show" -l tree -d 'Print the extend graph as a tree instead of a flat skill list'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from show" -s u -l user -d 'Use user-level config when checking which profile is active'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from show" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from show" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from show" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from show" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from rm" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from rm" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from rm" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from rm" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from rm" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from rm" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from help" -f -a "setup" -d 'Choose skills for a profile (interactive; creates the profile if missing)'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from help" -f -a "extend" -d 'Choose which profiles this one inherits skills from (interactive; creates the profile if missing)'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from help" -f -a "ls" -d 'List profile names'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from help" -f -a "show" -d 'Show skills in a profile'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from help" -f -a "rm" -d 'Remove a profile'
complete -c skm -n "__fish_skm_using_subcommand profile; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c skm -n "__fish_skm_using_subcommand skill; and not __fish_seen_subcommand_from ls setup rm validate help" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand skill; and not __fish_seen_subcommand_from ls setup rm validate help" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand skill; and not __fish_seen_subcommand_from ls setup rm validate help" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand skill; and not __fish_seen_subcommand_from ls setup rm validate help" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand skill; and not __fish_seen_subcommand_from ls setup rm validate help" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand skill; and not __fish_seen_subcommand_from ls setup rm validate help" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand skill; and not __fish_seen_subcommand_from ls setup rm validate help" -f -a "ls" -d 'List enabled skills in the store'
complete -c skm -n "__fish_skm_using_subcommand skill; and not __fish_seen_subcommand_from ls setup rm validate help" -f -a "setup" -d 'Choose which store skills are enabled (interactive; all enabled by default)'
complete -c skm -n "__fish_skm_using_subcommand skill; and not __fish_seen_subcommand_from ls setup rm validate help" -f -a "rm" -d 'Permanently remove a skill from the store'
complete -c skm -n "__fish_skm_using_subcommand skill; and not __fish_seen_subcommand_from ls setup rm validate help" -f -a "validate" -d 'Validate SKILL.md frontmatter against the Agent Skills spec'
complete -c skm -n "__fish_skm_using_subcommand skill; and not __fish_seen_subcommand_from ls setup rm validate help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from ls" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from ls" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from ls" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from ls" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from ls" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from ls" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from setup" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from setup" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from setup" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from setup" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from setup" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from setup" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from rm" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from rm" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from rm" -l force -d 'Remove without confirmation (required when stdin is not a TTY)'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from rm" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from rm" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from rm" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from rm" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from validate" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from validate" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from validate" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from validate" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from validate" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from validate" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from help" -f -a "ls" -d 'List enabled skills in the store'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from help" -f -a "setup" -d 'Choose which store skills are enabled (interactive; all enabled by default)'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from help" -f -a "rm" -d 'Permanently remove a skill from the store'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from help" -f -a "validate" -d 'Validate SKILL.md frontmatter against the Agent Skills spec'
complete -c skm -n "__fish_skm_using_subcommand skill; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c skm -n "__fish_skm_using_subcommand use-profiles" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand use-profiles" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand use-profiles" -s u -l user -d 'Use `~/.skm.toml` even when `./.skm.toml` exists'
complete -c skm -n "__fish_skm_using_subcommand use-profiles" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand use-profiles" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand use-profiles" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand use-profiles" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c skm -n "__fish_skm_using_subcommand add-profile" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand add-profile" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand add-profile" -s u -l user -d 'Use `~/.skm.toml` even when `./.skm.toml` exists'
complete -c skm -n "__fish_skm_using_subcommand add-profile" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand add-profile" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand add-profile" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand add-profile" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand remove-profile" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand remove-profile" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand remove-profile" -s u -l user -d 'Use `~/.skm.toml` even when `./.skm.toml` exists'
complete -c skm -n "__fish_skm_using_subcommand remove-profile" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand remove-profile" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand remove-profile" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand remove-profile" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand use-agents" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand use-agents" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand use-agents" -s u -l user -d 'Use `~/.skm.toml` even when `./.skm.toml` exists'
complete -c skm -n "__fish_skm_using_subcommand use-agents" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand use-agents" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand use-agents" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand use-agents" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c skm -n "__fish_skm_using_subcommand add-agent" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand add-agent" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand add-agent" -s u -l user -d 'Use `~/.skm.toml` even when `./.skm.toml` exists'
complete -c skm -n "__fish_skm_using_subcommand add-agent" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand add-agent" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand add-agent" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand add-agent" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c skm -n "__fish_skm_using_subcommand remove-agent" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand remove-agent" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand remove-agent" -s u -l user -d 'Use `~/.skm.toml` even when `./.skm.toml` exists'
complete -c skm -n "__fish_skm_using_subcommand remove-agent" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand remove-agent" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand remove-agent" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand remove-agent" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c skm -n "__fish_skm_using_subcommand destroy" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand destroy" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand destroy" -l force -d 'Destroy without confirmation (required when stdin is not a TTY)'
complete -c skm -n "__fish_skm_using_subcommand destroy" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand destroy" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand destroy" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand destroy" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c skm -n "__fish_skm_using_subcommand sync" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand sync" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand sync" -s u -l user -d 'Use `~/.skm.toml` even when `./.skm.toml` exists'
complete -c skm -n "__fish_skm_using_subcommand sync" -l no-pull -d 'Skip pulling registered remote repositories before reconcile'
complete -c skm -n "__fish_skm_using_subcommand sync" -l strict -d 'Fail when SKILL.md frontmatter is invalid (default: warn)'
complete -c skm -n "__fish_skm_using_subcommand sync" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand sync" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand sync" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand sync" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand status" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand status" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand status" -s u -l user -d 'Use `~/.skm.toml` even when `./.skm.toml` exists'
complete -c skm -n "__fish_skm_using_subcommand status" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand status" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand status" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand status" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c skm -n "__fish_skm_using_subcommand ls" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand ls" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand ls" -s p -l profile -d 'List profiles only (same as `skm profile ls`)'
complete -c skm -n "__fish_skm_using_subcommand ls" -s s -l skill -d 'List skills only (same as `skm skill ls`)'
complete -c skm -n "__fish_skm_using_subcommand ls" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand ls" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand ls" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand ls" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand scan" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand scan" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand scan" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand scan" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand scan" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand scan" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand search" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand search" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand search" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand search" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand search" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand search" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand doctor" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand doctor" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand doctor" -s u -l user -d 'Use `~/.skm.toml` even when `./.skm.toml` exists'
complete -c skm -n "__fish_skm_using_subcommand doctor" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand doctor" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand doctor" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand doctor" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand repo; and not __fish_seen_subcommand_from add ls rm pin browse help" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand repo; and not __fish_seen_subcommand_from add ls rm pin browse help" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand repo; and not __fish_seen_subcommand_from add ls rm pin browse help" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand repo; and not __fish_seen_subcommand_from add ls rm pin browse help" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand repo; and not __fish_seen_subcommand_from add ls rm pin browse help" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand repo; and not __fish_seen_subcommand_from add ls rm pin browse help" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand repo; and not __fish_seen_subcommand_from add ls rm pin browse help" -f -a "add" -d 'Clone and register a git repository as a skill source'
complete -c skm -n "__fish_skm_using_subcommand repo; and not __fish_seen_subcommand_from add ls rm pin browse help" -f -a "ls" -d 'List registered remote repositories'
complete -c skm -n "__fish_skm_using_subcommand repo; and not __fish_seen_subcommand_from add ls rm pin browse help" -f -a "rm" -d 'Remove a registered remote repository from the store'
complete -c skm -n "__fish_skm_using_subcommand repo; and not __fish_seen_subcommand_from add ls rm pin browse help" -f -a "pin" -d 'Pin a remote to a branch, tag, or commit (or show/clear the pin)'
complete -c skm -n "__fish_skm_using_subcommand repo; and not __fish_seen_subcommand_from add ls rm pin browse help" -f -a "browse" -d 'Browse the skills.sh leaderboard and register selected repositories'
complete -c skm -n "__fish_skm_using_subcommand repo; and not __fish_seen_subcommand_from add ls rm pin browse help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from add" -l name -d 'Registry and library prefix (default: derived from URL)' -r
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from add" -l pin -d 'Branch, tag, or commit to check out (default: repository default branch)' -r
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from add" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from add" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from add" -l strict -d 'Fail when SKILL.md frontmatter is invalid (default: warn)'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from add" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from add" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from add" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from add" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from ls" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from ls" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from ls" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from ls" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from ls" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from ls" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from rm" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from rm" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from rm" -l force -d 'Remove even when profiles reference skills under this repo'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from rm" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from rm" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from rm" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from rm" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from pin" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from pin" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from pin" -l clear -d 'Stop pinning; track the repository default branch on pull'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from pin" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from pin" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from pin" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from pin" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from browse" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from browse" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from browse" -l strict -d 'Fail when SKILL.md frontmatter is invalid (default: warn)'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from browse" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from browse" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from browse" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from browse" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from help" -f -a "add" -d 'Clone and register a git repository as a skill source'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from help" -f -a "ls" -d 'List registered remote repositories'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from help" -f -a "rm" -d 'Remove a registered remote repository from the store'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from help" -f -a "pin" -d 'Pin a remote to a branch, tag, or commit (or show/clear the pin)'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from help" -f -a "browse" -d 'Browse the skills.sh leaderboard and register selected repositories'
complete -c skm -n "__fish_skm_using_subcommand repo; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c skm -n "__fish_skm_using_subcommand update" -l store -d 'Store root directory (env: SKM_STORE)' -r -F
complete -c skm -n "__fish_skm_using_subcommand update" -l color -d 'When to colorize human output (`auto` respects NO_COLOR)' -r -f -a "auto\t''
always\t''
never\t''"
complete -c skm -n "__fish_skm_using_subcommand update" -s v -l verbose -d 'Enable verbose logging on stderr'
complete -c skm -n "__fish_skm_using_subcommand update" -l json -d 'Emit machine-readable JSON on stdout (`status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, `repo ls` only)'
complete -c skm -n "__fish_skm_using_subcommand update" -l dry-run -d 'Preview changes without writing (`sync`, `add-profile`, `remove-profile`, `skill rm`, `destroy` only)'
complete -c skm -n "__fish_skm_using_subcommand update" -s h -l help -d 'Print help'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profiles add-profile remove-profile use-agents add-agent remove-agent destroy sync status ls scan search doctor repo update help" -f -a "init" -d 'Set up the skill store and project config (`.skm.toml`)'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profiles add-profile remove-profile use-agents add-agent remove-agent destroy sync status ls scan search doctor repo update help" -f -a "import" -d 'Import a skill directory into the store (or `github:owner/repo` to register a remote)'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profiles add-profile remove-profile use-agents add-agent remove-agent destroy sync status ls scan search doctor repo update help" -f -a "profile" -d 'Create and manage profiles'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profiles add-profile remove-profile use-agents add-agent remove-agent destroy sync status ls scan search doctor repo update help" -f -a "skill" -d 'Manage skills in the store'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profiles add-profile remove-profile use-agents add-agent remove-agent destroy sync status ls scan search doctor repo update help" -f -a "use-profiles" -d 'Choose active profiles and sync skill links (interactive)'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profiles add-profile remove-profile use-agents add-agent remove-agent destroy sync status ls scan search doctor repo update help" -f -a "add-profile" -d 'Add a profile to the active set and sync skill links'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profiles add-profile remove-profile use-agents add-agent remove-agent destroy sync status ls scan search doctor repo update help" -f -a "remove-profile" -d 'Remove a profile from the active set and sync skill links'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profiles add-profile remove-profile use-agents add-agent remove-agent destroy sync status ls scan search doctor repo update help" -f -a "use-agents" -d 'Choose target agents interactively'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profiles add-profile remove-profile use-agents add-agent remove-agent destroy sync status ls scan search doctor repo update help" -f -a "add-agent" -d 'Add a target agent to this setup'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profiles add-profile remove-profile use-agents add-agent remove-agent destroy sync status ls scan search doctor repo update help" -f -a "remove-agent" -d 'Remove a target agent from this setup'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profiles add-profile remove-profile use-agents add-agent remove-agent destroy sync status ls scan search doctor repo update help" -f -a "destroy" -d 'Tear down this project\'s skm setup (`.skm.toml` and store-owned links)'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profiles add-profile remove-profile use-agents add-agent remove-agent destroy sync status ls scan search doctor repo update help" -f -a "sync" -d 'Refresh skill links without changing the active profiles'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profiles add-profile remove-profile use-agents add-agent remove-agent destroy sync status ls scan search doctor repo update help" -f -a "status" -d 'Show target agents, active profiles, linked skills, and name conflicts'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profiles add-profile remove-profile use-agents add-agent remove-agent destroy sync status ls scan search doctor repo update help" -f -a "ls" -d 'List skills and profiles in the store'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profiles add-profile remove-profile use-agents add-agent remove-agent destroy sync status ls scan search doctor repo update help" -f -a "scan" -d 'Refresh the on-disk skill index'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profiles add-profile remove-profile use-agents add-agent remove-agent destroy sync status ls scan search doctor repo update help" -f -a "search" -d 'Search indexed skill IDs and descriptions'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profiles add-profile remove-profile use-agents add-agent remove-agent destroy sync status ls scan search doctor repo update help" -f -a "doctor" -d 'Read-only health report for the store, profiles, and skill links'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profiles add-profile remove-profile use-agents add-agent remove-agent destroy sync status ls scan search doctor repo update help" -f -a "repo" -d 'Manage remote skill repositories (GitHub, GitLab, and other git hosts)'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profiles add-profile remove-profile use-agents add-agent remove-agent destroy sync status ls scan search doctor repo update help" -f -a "update" -d 'Pull registered remote repositories (no agent symlink changes)'
complete -c skm -n "__fish_skm_using_subcommand help; and not __fish_seen_subcommand_from init import profile skill use-profiles add-profile remove-profile use-agents add-agent remove-agent destroy sync status ls scan search doctor repo update help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c skm -n "__fish_skm_using_subcommand help; and __fish_seen_subcommand_from profile" -f -a "setup" -d 'Choose skills for a profile (interactive; creates the profile if missing)'
complete -c skm -n "__fish_skm_using_subcommand help; and __fish_seen_subcommand_from profile" -f -a "extend" -d 'Choose which profiles this one inherits skills from (interactive; creates the profile if missing)'
complete -c skm -n "__fish_skm_using_subcommand help; and __fish_seen_subcommand_from profile" -f -a "ls" -d 'List profile names'
complete -c skm -n "__fish_skm_using_subcommand help; and __fish_seen_subcommand_from profile" -f -a "show" -d 'Show skills in a profile'
complete -c skm -n "__fish_skm_using_subcommand help; and __fish_seen_subcommand_from profile" -f -a "rm" -d 'Remove a profile'
complete -c skm -n "__fish_skm_using_subcommand help; and __fish_seen_subcommand_from skill" -f -a "ls" -d 'List enabled skills in the store'
complete -c skm -n "__fish_skm_using_subcommand help; and __fish_seen_subcommand_from skill" -f -a "setup" -d 'Choose which store skills are enabled (interactive; all enabled by default)'
complete -c skm -n "__fish_skm_using_subcommand help; and __fish_seen_subcommand_from skill" -f -a "rm" -d 'Permanently remove a skill from the store'
complete -c skm -n "__fish_skm_using_subcommand help; and __fish_seen_subcommand_from skill" -f -a "validate" -d 'Validate SKILL.md frontmatter against the Agent Skills spec'
complete -c skm -n "__fish_skm_using_subcommand help; and __fish_seen_subcommand_from repo" -f -a "add" -d 'Clone and register a git repository as a skill source'
complete -c skm -n "__fish_skm_using_subcommand help; and __fish_seen_subcommand_from repo" -f -a "ls" -d 'List registered remote repositories'
complete -c skm -n "__fish_skm_using_subcommand help; and __fish_seen_subcommand_from repo" -f -a "rm" -d 'Remove a registered remote repository from the store'
complete -c skm -n "__fish_skm_using_subcommand help; and __fish_seen_subcommand_from repo" -f -a "pin" -d 'Pin a remote to a branch, tag, or commit (or show/clear the pin)'
complete -c skm -n "__fish_skm_using_subcommand help; and __fish_seen_subcommand_from repo" -f -a "browse" -d 'Browse the skills.sh leaderboard and register selected repositories'
