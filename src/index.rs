use std::{
    collections::HashMap,
    fs::{self, File},
    io::BufReader,
    path::{Path, PathBuf},
};

use xml::{
    EventReader,
    reader::{self, XmlEvent},
};

use crate::index::lexer::Lexer;

pub mod lexer;
pub mod read;
pub mod write;

pub type TermFreq = HashMap<String, usize>;
pub type TermFreqIndex = HashMap<PathBuf, TermFreq>;

fn extract_text_from_xml<P: AsRef<Path>>(file_path: P) -> Result<String, reader::Error> {
    let file = File::open(file_path)?;
    let event_reader = EventReader::new(BufReader::new(file));

    let mut content = String::new();
    for event in event_reader {
        if let XmlEvent::Characters(text) = event? {
            content.push_str(&text);
            content.push_str(" ");
        }
    }
    Ok(content)
}

pub fn compute_term_freq(content: &str, to_term: impl Fn(&str) -> String) -> TermFreq {
    Lexer::new(content)
        .into_iter()
        .map(to_term)
        .fold(HashMap::new(), |mut acc, term| {
            *acc.entry(term).or_insert(0) += 1;
            acc
        })
}

pub fn index_directory(
    dir_path: &Path,
    recursive: bool,
    to_term: impl Fn(&str) -> String,
) -> TermFreqIndex {
    let mut term_freq_index = HashMap::new();
    index_directory_rec(dir_path, recursive, &mut term_freq_index, &to_term);
    term_freq_index
}

fn index_directory_rec(
    dir_path: &Path,
    recursive: bool,
    term_freq_index: &mut TermFreqIndex,
    to_term: &impl Fn(&str) -> String,
) {
    if !dir_path.is_dir() {
        eprintln!("{dir_path:?} is not dir, skip");
        return;
    }

    let entries = fs::read_dir(dir_path)
        .expect("Read dir failure")
        .filter_map(|entry| match entry {
            Ok(entry) => Some(entry),
            Err(e) => {
                eprintln!("Read entry failure: {:?}", e);
                None
            }
        });

    for entry in entries {
        let path = entry.path();

        if path
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.starts_with('.'))
            .unwrap_or(false)
        {
            continue;
        }

        // 获取元数据，并且不自动追踪软链接
        let is_symlink = path
            .symlink_metadata()
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false);

        if is_symlink {
            eprintln!("Warning: {path:?} is a symlink, skipping to avoid infinite loops.");
            continue;
        }

        if path.is_dir() {
            if recursive {
                index_directory_rec(&path, recursive, term_freq_index, to_term)
            }
        } else {
            println!("Indexing {path:?}");
            match extract_text_from_xml(&path) {
                Ok(content) => {
                    term_freq_index.insert(path, compute_term_freq(&content, to_term));
                }
                Err(e) => {
                    eprintln!("Error reading file {path:?}: {e:?}");
                }
            }
        }
    }
}
