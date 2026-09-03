use clap::FromArgMatches;
use skill_manager::{cli_command, exit_code_from_error, init_logging, print_error, run, Cli};

fn main() {
    let matches = cli_command().get_matches();
    let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|err| err.exit());
    let verbose = cli.verbose;
    init_logging(verbose);

    match run(cli) {
        Ok(exit) => {
            if exit != 0 {
                std::process::exit(exit);
            }
        }
        Err(err) => {
            print_error(&err, verbose);
            std::process::exit(exit_code_from_error(&err));
        }
    }
}
