use std::io;

use crate::index::{TermFreqIndex, index_directory, read::read_index, write::write_index};
use clap::{Parser, Subcommand};

mod index;

#[derive(Debug, Parser)]
struct Args {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Index a directory of XML files
    Index {
        /// The directory to index
        #[arg(short, long)]
        dir: String,
        /// The output file path
        #[arg(short, long, default_value = "indexes/index.json")]
        output: String,
    },
    /// Read an index from a JSON file
    Read {
        #[arg(short, long)]
        path: String,
    },
}

fn main() -> Result<(), io::Error> {
    let args = Args::parse();

    match args.commands {
        Commands::Index { dir, output } => {
            let mut term_freq_index = TermFreqIndex::new();
            term_freq_index.extend(index_directory(&dir)?);
            write_index(&term_freq_index, &output)?;
            println!("Saved {}", output);
        }
        Commands::Read { path } => {
            let term_freq_index = read_index(&path)?;

            println!(
                "{path} contains {count} files",
                count = term_freq_index.len()
            );
        }
    }

    Ok(())
}
