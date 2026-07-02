//! `rendertree` binary entry point.

use std::io::{self, Write};
use std::process::ExitCode;

use spannerplan_cli::{run, UsageError};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();

    let stdin = io::stdin();
    let mut stdin = stdin.lock();
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();

    match run(&arg_refs, &mut stdin, &mut stdout, &mut stderr) {
        Ok(_) => ExitCode::SUCCESS,
        Err(err) if err.downcast_ref::<UsageError>().is_some() => {
            let _ = writeln!(stderr, "{err}");
            ExitCode::from(2)
        }
        Err(err) => {
            let _ = writeln!(stderr, "{err}");
            ExitCode::FAILURE
        }
    }
}
