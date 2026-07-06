use std::fmt::Display;
use std::{
    fs::{self, File},
    io::BufReader,
    path::Path,
};

use xml::{
    EventReader,
    reader::{self, XmlEvent},
};

use crate::index::model::Model;
use crate::index::term_processor::TermProcessor;

pub mod lexer;
pub mod model;
pub mod read;
pub mod term_processor;
pub mod write;

fn extract_text_from_txt<P: AsRef<Path>>(file_path: P) -> Result<String, std::io::Error> {
    fs::read_to_string(file_path)
}

fn extract_text_from_pdf<P: AsRef<Path>>(file_path: P) -> Result<String, pdf_extract::OutputError> {
    pdf_extract::extract_text(file_path)
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

fn extract_text_from_file_by_extensions<P: AsRef<Path>>(file_path: P) -> Option<String> {
    let file_path = file_path.as_ref();
    let extension = file_path
        .extension()
        .map(|s| s.to_string_lossy().to_lowercase());

    let handle_err = |ext: &str, err: &dyn Display| {
        eprintln!(
            "ERROR: Failed to read {} file {}: {}",
            ext.to_uppercase(),
            file_path.display(),
            err
        );
    };
    match extension.as_deref() {
        Some("txt") => extract_text_from_txt(file_path)
            .inspect_err(|e| handle_err("txt", e))
            .ok(),
        Some("pdf") => extract_text_from_pdf(file_path)
            .inspect_err(|e| handle_err("pdf", e))
            .ok(),
        Some("xhtml" | "html" | "xml") => extract_text_from_xml(file_path)
            .inspect_err(|e| handle_err("xml", e))
            .ok(),
        Some(ext) => {
            eprintln!(
                "ERROR: can't detect file type of {file_path}: unsupported extension {extension}",
                file_path = file_path.display(),
                extension = ext
            );
            None
        }
        None => {
            eprintln!(
                "ERROR: can't detect file type of {file_path} without extension",
                file_path = file_path.display()
            );
            None
        }
    }
}

pub fn index_directory(
    dir_path: &Path,
    recursive: bool,
    term_processor: &impl TermProcessor,
) -> Model {
    let mut model = Model::default();
    index_directory_rec(dir_path, recursive, &mut model, term_processor);
    model
}

fn index_directory_rec(
    dir_path: &Path,
    recursive: bool,
    model: &mut Model,
    term_processor: &impl TermProcessor,
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
                index_directory_rec(&path, recursive, model, term_processor)
            }
        } else {
            println!("Indexing {path:?}");
            let last_modified = match path.metadata().and_then(|m| m.modified()) {
                Ok(time) => time,
                Err(err) => {
                    eprintln!("ERROR: could not get the modification time for {path:?}: {err}");
                    continue;
                }
            };
            match extract_text_from_file_by_extensions(&path) {
                Some(content) => {
                    model.index_doc(path, last_modified, &content, term_processor);
                }
                None => {
                    eprintln!("Error reading file {path:?}");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::index::{extract_text_from_pdf, extract_text_from_txt};

    #[test]
    fn test_extract_text_from_pdf() {
        let path = std::path::Path::new("tests/fixtures/test.pdf");
        assert_eq!(
            "With great power comes great responsibility ^_~",
            extract_text_from_pdf(&path)
                .as_deref()
                .map(|s| s.trim())
                .unwrap_or("")
        );
    }

    #[test]
    fn test_extract_text_from_txt() {
        let path = std::path::Path::new("tests/fixtures/test.txt");
        assert_eq!(
            "With great power comes great responsibility ^_~",
            extract_text_from_txt(&path)
                .as_deref()
                .map(|s| s.trim())
                .unwrap_or("")
        );
    }
}
