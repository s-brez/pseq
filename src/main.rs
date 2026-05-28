use clap::Parser;

use pseq::cli::Cli;
use pseq::error::{write_cli_error, write_error};

fn main() {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            if wants_json_output() {
                let _ = write_cli_error(&error);
                std::process::exit(error.exit_code());
            }
            error.exit();
        }
    };
    let json = cli.json;
    let quiet = cli.quiet;

    let exit_code = match pseq::run(cli) {
        Ok(result) => {
            if let Err(error) = result.payload.write_to_stdout(json, quiet) {
                let _ = write_error(&error, json);
                error.exit_code()
            } else {
                result.exit_code
            }
        }
        Err(error) => {
            let exit_code = error.exit_code();
            let _ = write_error(&error, json);
            exit_code
        }
    };

    std::process::exit(exit_code);
}

fn wants_json_output() -> bool {
    std::env::args_os().any(|arg| arg == "--json")
}
