use std::{
    collections::HashMap,
    fs::{self, File},
    path::{Path, PathBuf},
};

use xml::{
    EventReader,
    reader::{self, XmlEvent},
};

pub mod read;
pub mod write;

pub type TermFreq = HashMap<String, usize>;
pub type TermFreqIndex = HashMap<PathBuf, TermFreq>;

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

fn read_entire_xml_file<P: AsRef<Path>>(file_path: P) -> Result<String, reader::Error> {
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

pub fn index_document(content: &str) -> TermFreq {
    Lexer::new(content.chars().collect::<Vec<_>>().as_slice())
        .into_iter()
        .map(|token| token.iter().map(|c| c.to_ascii_uppercase()).collect())
        .fold(HashMap::new(), |mut acc, term| {
            *acc.entry(term).or_insert(0) += 1;
            acc
        })
}

pub fn index_directory(dir_path: &Path, recursive: bool) -> HashMap<PathBuf, TermFreq> {
    let mut term_freq_index = HashMap::new();
    index_directory_impl(dir_path, recursive, &mut term_freq_index);
    term_freq_index
}

pub fn index_directory_impl(
    dir_path: &Path,
    recursive: bool,
    term_freq_index: &mut HashMap<PathBuf, TermFreq>,
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
                index_directory_impl(&path, recursive, term_freq_index)
            }
        } else {
            println!("Indexing {path:?}");
            match read_entire_xml_file(&path) {
                Ok(content) => {
                    term_freq_index.insert(path, index_document(&content));
                }
                Err(e) => {
                    eprintln!("Error reading file {path:?}: {e:?}");
                }
            }
        }
    }
}
