use crate::command::{Args, CommandError};
use clap::Parser;

mod command;
mod index;

fn main() -> Result<(), CommandError> {
    let args = Args::parse();
    command::handle(args)?;
    Ok(())
}
