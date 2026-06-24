use std::path::PathBuf;

use crate::index::{Model, compute_term_freq};

pub trait Searcher {
    fn search<'a>(
        &'a self,
        keywords: &str,
        strategy: impl Fn(&str) -> String,
    ) -> Vec<(&'a PathBuf, f32)>;
}

/// Searcher implementation for searching using TF-IDF algorithm.
pub struct TfIdfSearcher {
    pub model: Model,
}

impl TfIdfSearcher {
    pub fn new(model: Model) -> Self {
        TfIdfSearcher { model }
    }

    fn idf(&self, term: &str) -> f32 {
        let n = self.model.doc_count as f32;
        if n == 0.0 {
            return 0.0;
        }
        let doc_count = self.model.doc_freq.get(term).copied().unwrap_or(0) as f32;
        ((n + 1.0) / (doc_count + 1.0)).ln() + 1.0
    }
}

impl Searcher for TfIdfSearcher {
    fn search<'a>(
        &'a self,
        keywords: &str,
        strategy: impl Fn(&str) -> String,
    ) -> Vec<(&'a PathBuf, f32)> {
        let term_freq = compute_term_freq(&keywords, strategy);
        let query_terms = term_freq
            .into_iter()
            .map(|(token, qtf)| {
                let idf = self.idf(&token);
                let qtf_score = 1.0 + (qtf as f32).ln();
                (token, idf, qtf_score)
            })
            .collect::<Vec<_>>();
        let mut result = Vec::new();

        for (path, doc_info) in &self.model.docs {
            let doc_term_count = doc_info.term_count;
            if doc_term_count == 0 {
                continue;
            }
            let doc_term_count = doc_term_count as f32;

            let mut rank = 0f32;
            for (token, idf, qtf_score) in &query_terms {
                if let Some(&count) = doc_info.term_freq.get(token) {
                    let tf = count as f32 / doc_term_count;
                    rank += tf * idf * qtf_score;
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
    model: Model,
    k1: f32,
    b: f32,
    k3: f32,
}

impl BM25Searcher {
    pub fn new(model: Model) -> Self {
        Self {
            model,
            k1: 1.5,
            b: 0.75,
            k3: 1.2,
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

    pub fn with_k3(mut self, k3: f32) -> Self {
        self.k3 = k3;
        self
    }

    fn idf(&self, term: &str) -> f32 {
        let total_docs = self.model.doc_count as f32;
        if total_docs == 0.0 {
            return 0.0;
        }
        let doc_count = self.model.doc_freq.get(term).copied().unwrap_or(0) as f32;

        ((total_docs - doc_count + 0.5) / (doc_count + 0.5) + 1.0).ln()
    }
}

impl Searcher for BM25Searcher {
    fn search<'a>(
        &'a self,
        keywords: &str,
        strategy: impl Fn(&str) -> String,
    ) -> Vec<(&'a PathBuf, f32)> {
        let term_freq = compute_term_freq(&keywords, strategy);
        let query_terms = term_freq
            .into_iter()
            .map(|(token, qtf)| {
                let idf = self.idf(&token);
                let qtf = qtf as f32;
                let qtf_score = (self.k3 + 1.0) * qtf / (self.k3 + qtf);
                (token, idf, qtf_score)
            })
            .collect::<Vec<_>>();

        let mut result = Vec::new();

        for (path, doc_info) in &self.model.docs {
            let doc_term_count = doc_info.term_count as f32;

            let mut rank = 0f32;
            for (token, idf, qtf_score) in &query_terms {
                if let Some(&count) = doc_info.term_freq.get(token) {
                    let tf = count as f32;
                    let tf_score = tf * (self.k1 + 1.0)
                        / (tf
                            + self.k1
                                * (1.0 - self.b
                                    + self.b * doc_term_count / self.model.avg_term_count));
                    rank += tf_score * idf * qtf_score;
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
