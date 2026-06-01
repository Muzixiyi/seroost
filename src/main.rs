use std::collections::HashMap;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use thiserror::Error;
use xml::reader::XmlEvent;
use xml::{EventReader, reader};

type TermFreq = HashMap<String, usize>;
type TermFreqIndex = HashMap<PathBuf, TermFreq>;

#[derive(Error, Debug)]
pub enum IndexXmlFileError {
    #[error("io error: {0}")]
    IoError(#[from] io::Error),
    #[error("xml error: {0}")]
    XmlError(#[from] reader::Error),
    #[error("json error: {0}")]
    JsonError(#[from] serde_json::Error),
}

fn read_entire_xml_file<P: AsRef<Path>>(file_path: P) -> Result<String, IndexXmlFileError> {
    let file = File::open(file_path)?;
    let event_reader = EventReader::new(file);

    let mut content = String::new();
    for event in event_reader {
        if let XmlEvent::Characters(text) = event? {
            content.push_str(&text);
            content.push_str(" ");
        }
    }
    Ok(content)
}

#[derive(Debug)]
struct Lexer<'a> {
    content: &'a [char],
}

impl<'a> Lexer<'a> {
    fn new(content: &'a [char]) -> Self {
        Self { content }
    }

    fn trim_left(&mut self) {
        while self.content.len() > 0 && self.content[0].is_whitespace() {
            self.content = &self.content[1..];
        }
    }

    fn chop(&mut self, n: usize) -> &'a [char] {
        let token = &self.content[0..n];
        self.content = &self.content[n..];
        token
    }

    fn chop_while(&mut self, predicate: impl Fn(&char) -> bool) -> &'a [char] {
        let mut n = 0;
        while n < self.content.len() && predicate(&self.content[n]) {
            n += 1;
        }
        self.chop(n)
    }

    fn next_token(&mut self) -> Option<&'a [char]> {
        self.trim_left();
        if self.content.len() == 0 {
            return None;
        }

        if self.content[0].is_numeric() {
            return Some(self.chop_while(|x| x.is_numeric()));
        }

        if self.content[0].is_alphabetic() {
            return Some(self.chop_while(|x| x.is_alphanumeric()));
        }

        Some(self.chop(1))
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = &'a [char];

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}

fn index_document(content: &str) -> TermFreq {
    Lexer::new(content.chars().collect::<Vec<_>>().as_slice())
        .into_iter()
        .map(|token| token.iter().map(|c| c.to_ascii_uppercase()).collect())
        .fold(HashMap::new(), |mut acc, term| {
            *acc.entry(term).or_insert(0) += 1;
            acc
        })
}

fn index_directory(dir_path: &str) -> Result<TermFreqIndex, IndexXmlFileError> {
    let entries = fs::read_dir(dir_path)?.collect::<Result<Vec<_>, _>>()?;
    Ok(entries
        .iter()
        .filter_map(|entry| {
            let file_path = entry.path();

            println!("Indexing {file_path:?}");

            if let Ok(content) = read_entire_xml_file(&file_path) {
                Some((file_path, index_document(&content)))
            } else {
                eprintln!("Error reading file {file_path:?}: e");
                None
            }
        })
        .collect())
}

fn write_index(index: &TermFreqIndex, path: &str) -> Result<(), IndexXmlFileError> {
    let path = Path::new(path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = File::create(path)?;
    serde_json::to_writer_pretty(&file, index)?;
    Ok(())
}

fn read_index(path: &str) -> Result<TermFreqIndex, IndexXmlFileError> {
    let file = File::open(path)?;
    let index = serde_json::from_reader(file)?;
    Ok(index)
}

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

fn main() -> Result<(), IndexXmlFileError> {
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
