use std::path::PathBuf;

use crate::index::{TermFreqIndex, lexer::Lexer};

pub trait Searcher {
    fn search<'a>(
        &'a self,
        keywords: &str,
        strategy: impl Fn(&str) -> String,
    ) -> Vec<(&'a PathBuf, f32)>;
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
        ((n + 1.0) / (doc_count + 1.0)).ln() + 1.0
    }
}

impl Searcher for TfIdfSearcher {
    fn search<'a>(
        &'a self,
        keywords: &str,
        strategy: impl Fn(&str) -> String,
    ) -> Vec<(&'a PathBuf, f32)> {
        let keywords = strategy(keywords);

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

/// Searcher implementation for searching using BM25 algorithm.
pub struct BM25Searcher {
    indexes: TermFreqIndex,
    avg_doc_len: f32,
    k1: f32,
    b: f32,
}

impl BM25Searcher {
    pub fn new(indexes: TermFreqIndex) -> Self {
        let avg_doc_len = if indexes.len() == 0 {
            0.0
        } else {
            indexes
                .values()
                .map(|tf| tf.values().sum::<usize>() as f32)
                .sum::<f32>()
                / indexes.len() as f32
        };
        Self {
            indexes,
            avg_doc_len,
            k1: 1.5,
            b: 0.75,
        }
    }

    pub fn with_k1(mut self, k1: f32) -> Self {
        self.k1 = k1;
        self
    }

    pub fn with_b(mut self, b: f32) -> Self {
        self.b = b;
        self
    }

    fn idf(&self, term: &str) -> f32 {
        let total_docs = self.indexes.len() as f32;
        if total_docs == 0.0 {
            return 0.0;
        }
        let doc_count = self
            .indexes
            .values()
            .filter(|term_freq| term_freq.contains_key(term))
            .count() as f32;

        ((total_docs - doc_count + 0.5) / (doc_count + 0.5) + 1.0).ln()
    }
}

impl Searcher for BM25Searcher {
    fn search<'a>(
        &'a self,
        keywords: &str,
        strategy: impl Fn(&str) -> String,
    ) -> Vec<(&'a PathBuf, f32)> {
        let keywords = strategy(keywords);

        let idfs = Lexer::new(&keywords)
            .map(|token| (token, self.idf(&token)))
            .collect::<Vec<_>>();

        let mut result = Vec::new();

        for (path, term_freq_table) in &self.indexes {
            let doc_len = term_freq_table.values().sum::<usize>() as f32;

            let mut rank = 0f32;
            for (token, idf) in &idfs {
                if let Some(&count) = term_freq_table.get(*token) {
                    let tf = count as f32;
                    let tf_score = tf * (self.k1 + 1.0)
                        / (tf + self.k1 * (1.0 - self.b + self.b * doc_len / self.avg_doc_len));
                    rank += tf_score * idf;
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
