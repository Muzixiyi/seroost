use std::path::PathBuf;

use crate::index::{TermFreqIndex, lexer::Lexer};

pub trait Searcher {
    fn search<'a>(&'a self, keywords: &str) -> Vec<(&'a PathBuf, f32)>;
}

/// Searcher implementation for searching using TF-IDF algorithm.
pub struct TfIdfSearcher {
    pub indexes: TermFreqIndex,
}

impl TfIdfSearcher {
    pub fn new(term_freq_indexes: TermFreqIndex) -> Self {
        TfIdfSearcher {
            indexes: term_freq_indexes,
        }
    }

    fn idf(&self, term: &str) -> f32 {
        let n = self.indexes.len() as f32;
        if n == 0.0 {
            return 0.0;
        }
        let doc_count = self
            .indexes
            .values()
            .filter(|term_freq| term_freq.contains_key(term))
            .count() as f32;
        ((n + 1.0) / (doc_count + 1.0)).log2() + 1.0
    }
}

impl Searcher for TfIdfSearcher {
    fn search<'a>(&'a self, keywords: &str) -> Vec<(&'a PathBuf, f32)> {
        let keywords = keywords.to_ascii_uppercase();

        let idfs = Lexer::new(&keywords)
            .map(|token| (token, self.idf(&token)))
            .collect::<Vec<_>>();

        let mut result = Vec::new();

        for (path, term_freq_table) in &self.indexes {
            let total_terms = term_freq_table.values().sum::<usize>();
            if total_terms == 0 {
                continue;
            }
            let total_terms = total_terms as f32;

            let mut rank = 0f32;
            for (token, idf) in &idfs {
                if let Some(&count) = term_freq_table.get(*token) {
                    let tf = count as f32 / total_terms;
                    rank += tf * idf;
                }
            }

            if rank > 0.0 {
                result.push((path, rank));
            }
        }

        result.sort_by(|(_, rank1), (_, rank2)| rank2.total_cmp(rank1));

        result
    }
}
