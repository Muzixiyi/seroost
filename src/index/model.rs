use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::SystemTime,
};

use serde::{Deserialize, Serialize};

use crate::index::{lexer::Lexer, term_processor::TermProcessor};

pub type TermFreq = HashMap<String, usize>;
pub type DocFreq = HashMap<String, usize>;
pub type DocRegistry = HashMap<PathBuf, DocInfo>;

#[derive(Default, Serialize, Deserialize)]
pub struct Model {
    doc_count: usize,
    total_term_count: usize,
    doc_freq: DocFreq,
    docs: DocRegistry,
}

#[derive(Serialize, Deserialize)]
pub struct DocInfo {
    pub term_count: usize,
    pub term_freq: TermFreq,
    pub last_modified: SystemTime,
}

impl Model {
    pub fn doc_count(&self) -> usize {
        self.doc_count
    }

    pub fn total_term_count(&self) -> usize {
        self.total_term_count
    }

    pub fn doc_freq(&self) -> &DocFreq {
        &self.doc_freq
    }

    pub fn docs(&self) -> &DocRegistry {
        &self.docs
    }

    pub fn avg_term_count(&self) -> f32 {
        if self.doc_count > 0 {
            self.total_term_count as f32 / self.doc_count as f32
        } else {
            0.0
        }
    }

    fn remove_doc<P: AsRef<Path>>(&mut self, path: P) {
        if let Some(doc) = self.docs.remove(path.as_ref()) {
            self.doc_count -= 1;
            self.total_term_count -= doc.term_count;

            for term in doc.term_freq.keys() {
                if let Some(count) = self.doc_freq.get_mut(term) {
                    *count -= 1;

                    if *count == 0 {
                        self.doc_freq.remove(term);
                    }
                }
            }
        }
    }

    pub fn is_stale<P: AsRef<Path>>(&self, path: P, last_modified: SystemTime) -> bool {
        if let Some(doc) = self.docs.get(path.as_ref()) {
            return doc.last_modified != last_modified;
        }
        true
    }

    pub fn index_doc<P: AsRef<Path>>(
        &mut self,
        path: P,
        last_modified: SystemTime,
        content: &str,
        term_processor: &impl TermProcessor,
    ) {
        let path = path.as_ref();
        self.remove_doc(path);

        let term_freq = compute_term_freq(content, term_processor);

        let mut term_count = 0;
        for (term, freq) in &term_freq {
            if let Some(count) = self.doc_freq.get_mut(term) {
                *count += 1;
            } else {
                self.doc_freq.insert(term.clone(), 1);
            }
            term_count += freq;
        }

        self.doc_count += 1;
        self.total_term_count += term_count;

        self.docs.insert(
            path.to_path_buf(),
            DocInfo {
                term_count,
                term_freq,
                last_modified,
            },
        );
    }
}

pub fn compute_term_freq(content: &str, term_processor: &impl TermProcessor) -> TermFreq {
    Lexer::new(content)
        .into_iter()
        .map(|s| term_processor.process(s))
        .fold(HashMap::new(), |mut acc, term| {
            if let Some(count) = acc.get_mut(term.as_ref()) {
                *count += 1;
            } else {
                acc.insert(term.into_owned(), 1);
            }
            acc
        })
}
