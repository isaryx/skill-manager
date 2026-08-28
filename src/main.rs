use clap::Parser;
use skill_manager::{exit_code_from_error, init_logging, print_error, run, Cli};

fn main() {
    let cli = Cli::parse();
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
