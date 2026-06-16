use clap::Parser;
use seroost::command::{Args, CommandError, handle};

fn main() -> Result<(), CommandError> {
    let args = Args::parse();
    handle(args)?;
    Ok(())
}
