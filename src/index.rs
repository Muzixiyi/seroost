use std::{
    collections::HashMap,
    fs::{self, File},
    io::BufReader,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use xml::{
    EventReader,
    reader::{self, XmlEvent},
};

use crate::index::lexer::Lexer;

pub mod lexer;
pub mod read;
pub mod write;

pub type TermFreq = HashMap<String, usize>;
pub type DocFreq = HashMap<String, usize>;
pub type DocRegistry = HashMap<PathBuf, DocInfo>;

#[derive(Default, Serialize, Deserialize)]
pub struct Model {
    pub doc_count: usize,
    pub total_term_count: usize,
    pub avg_term_count: f32,
    pub doc_freq: DocFreq,
    pub docs: DocRegistry,
}

#[derive(Default, Serialize, Deserialize)]
pub struct DocInfo {
    pub term_count: usize,
    pub term_freq: TermFreq,
}

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
) -> Model {
    let mut model = Model::default();
    index_directory_rec(dir_path, recursive, &mut model, &to_term);
    model.avg_term_count = if model.doc_count == 0 {
        0.0
    } else {
        (model.total_term_count as f32) / (model.doc_count as f32)
    };
    model
}

fn index_directory_rec(
    dir_path: &Path,
    recursive: bool,
    model: &mut Model,
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
                index_directory_rec(&path, recursive, model, to_term)
            }
        } else {
            println!("Indexing {path:?}");
            match extract_text_from_xml(&path) {
                Ok(content) => {
                    let term_freq = compute_term_freq(&content, to_term);
                    for term in term_freq.keys() {
                        let entry = model.doc_freq.entry(term.clone()).or_insert(0);
                        *entry += 1;
                    }
                    let term_count = term_freq.values().sum::<usize>();
                    model.doc_count += 1;
                    model.total_term_count += term_count;
                    model.docs.insert(
                        path,
                        DocInfo {
                            term_count,
                            term_freq,
                        },
                    );
                }
                Err(e) => {
                    eprintln!("Error reading file {path:?}: {e:?}");
                }
            }
        }
    }
}
