use super::*;

impl BM25Index {
    pub fn new() -> Self {
        Self {
            doc_freq: HashMap::new(),
            doc_count: 0,
            avg_doc_len: 0.0,
            doc_lengths: HashMap::new(),
            term_freqs: HashMap::new(),
            doc_sources: HashMap::new(),
            k1: 1.2,
            b: 0.75,
            enabled: true,
        }
    }

    pub fn add_document(&mut self, doc_id: &str, content: &str, source: &str) {
        if !self.enabled {
            return;
        }

        let terms = Self::tokenize(content);
        let doc_len = terms.len();

        self.doc_lengths.insert(doc_id.to_string(), doc_len);
        self.doc_sources
            .insert(doc_id.to_string(), source.to_string());

        let mut term_freq: HashMap<String, usize> = HashMap::new();
        let mut seen_terms: std::collections::HashSet<String> = std::collections::HashSet::new();

        for term in &terms {
            *term_freq.entry(term.clone()).or_insert(0) += 1;

            if !seen_terms.contains(term) {
                *self.doc_freq.entry(term.clone()).or_insert(0) += 1;
                seen_terms.insert(term.clone());
            }
        }

        self.term_freqs.insert(doc_id.to_string(), term_freq);
        self.doc_count += 1;

        let total_len: usize = self.doc_lengths.values().sum();
        self.avg_doc_len = total_len as f32 / self.doc_count as f32;
    }

    pub fn remove_document(&mut self, doc_id: &str) {
        if let Some(term_freq) = self.term_freqs.remove(doc_id) {
            for term in term_freq.keys() {
                if let Some(freq) = self.doc_freq.get_mut(term) {
                    *freq = freq.saturating_sub(1);
                    if *freq == 0 {
                        self.doc_freq.remove(term);
                    }
                }
            }
        }

        self.doc_lengths.remove(doc_id);
        self.doc_sources.remove(doc_id);
        self.doc_count = self.doc_count.saturating_sub(1);

        if self.doc_count > 0 {
            let total_len: usize = self.doc_lengths.values().sum();
            self.avg_doc_len = total_len as f32 / self.doc_count as f32;
        } else {
            self.avg_doc_len = 0.0;
        }
    }

    pub fn search(&self, query: &str, max_results: usize) -> Vec<(String, String, f32)> {
        if !self.enabled {
            return Vec::new();
        }

        let query_terms = Self::tokenize(query);
        let mut scores: HashMap<String, f32> = HashMap::new();

        for term in &query_terms {
            let df = *self.doc_freq.get(term).unwrap_or(&0);
            if df == 0 {
                continue;
            }

            let idf = ((self.doc_count as f32 - df as f32 + 0.5) / (df as f32 + 0.5)).ln_1p();

            for (doc_id, term_freqs) in &self.term_freqs {
                if let Some(&tf) = term_freqs.get(term) {
                    let doc_len = *self.doc_lengths.get(doc_id).unwrap_or(&1) as f32;
                    let tf_normalized = (tf as f32 * (self.k1 + 1.0))
                        / self.k1.mul_add(
                            self.b.mul_add(doc_len / self.avg_doc_len, 1.0 - self.b),
                            tf as f32,
                        );

                    *scores.entry(doc_id.clone()).or_insert(0.0) += idf * tf_normalized;
                }
            }
        }

        let mut results: Vec<(String, f32)> = scores.into_iter().collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(max_results);

        results
            .into_iter()
            .map(|(doc_id, score)| {
                let source = self.doc_sources.get(&doc_id).cloned().unwrap_or_default();
                (doc_id, source, score)
            })
            .collect()
    }

    pub(super) fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| s.len() > 2)
            .map(|s| s.to_string())
            .collect()
    }

    pub fn stats(&self) -> BM25Stats {
        BM25Stats {
            doc_count: self.doc_count,
            unique_terms: self.doc_freq.len(),
            avg_doc_len: self.avg_doc_len,
            enabled: self.enabled,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}
impl Default for BM25Index {
    fn default() -> Self {
        Self::new()
    }
}
