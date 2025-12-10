// mind-kb-rs/src/text_preprocessor.rs
// NLP Text Preprocessing Pipeline for Knowledge Base
// Tokenization, Stemming, and Stopword Removal for improved semantic search

use rust_stemmers::{Algorithm, Stemmer};
use unicode_segmentation::UnicodeSegmentation;
use std::collections::HashSet;
use once_cell::sync::Lazy;
use input_validation_rs::{ValidationResult, validate, validators::string::StringValidation};

// Constants for validation
const MAX_INPUT_LENGTH: usize = 1_048_576; // 1MB
const MAX_TOKENS: usize = 10_000; // Protect against tokenizer DoS

/// Common English stopwords
static STOPWORDS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "a", "an", "the", "and", "or", "but", "in", "on", "at", "to", "for",
        "of", "with", "by", "from", "as", "is", "was", "are", "were", "been",
        "be", "have", "has", "had", "do", "does", "did", "will", "would",
        "could", "should", "may", "might", "must", "shall", "can", "need",
        "it", "its", "this", "that", "these", "those", "he", "she", "they",
        "we", "you", "i", "my", "your", "his", "her", "their", "our",
        "what", "which", "who", "whom", "whose", "when", "where", "why", "how",
        "not", "no", "so", "if", "then", "than", "too", "very", "just",
        "about", "into", "through", "during", "before", "after", "above",
        "below", "between", "under", "again", "further", "once", "here",
        "there", "all", "each", "few", "more", "most", "other", "some",
        "such", "only", "own", "same", "also", "both", "any", "nor",
    ].iter().cloned().collect()
});

/// Preprocessing result
#[derive(Debug, Clone)]
pub struct PreprocessedText {
    pub original: String,
    pub tokens: Vec<String>,
    pub stems: Vec<String>,
    pub normalized: String,
}

/// Create a new English stemmer
fn create_stemmer() -> Stemmer {
    Stemmer::create(Algorithm::English)
}

/// Tokenize text into words
pub fn tokenize(text: &str) -> Vec<String> {
    // Validate input before tokenization to prevent ReDoS
    if let Err(e) = validate_text_input(text) {
        log::warn!("Invalid input for tokenization: {}", e);
        return Vec::new();
    }

    // Limit the number of tokens to prevent DoS
    text.unicode_words()
        .take(MAX_TOKENS)
        .map(|w| w.to_lowercase())
        .filter(|w| w.len() > 1) // Filter single characters
        .collect()
}

/// Validates text input to prevent ReDoS and other attacks
pub fn validate_text_input(text: &str) -> ValidationResult<()> {
    // Validate input length
    validate!(
        text,
        StringValidation::max_length(MAX_INPUT_LENGTH)
    )?;
    
    // Check for malformed UTF-8 sequences
    if text.chars().any(|c| (c as u32) == 0xFFFD) {
        return Err("Text contains invalid Unicode replacement characters".to_string());
    }
    
    Ok(())
}

/// Remove stopwords from tokens
pub fn remove_stopwords(tokens: &[String]) -> Vec<String> {
    // Protect against oversized token lists
    if tokens.len() > MAX_TOKENS {
        log::warn!("Token list too large for stopword removal: {}", tokens.len());
        return Vec::new();
    }

    tokens
        .iter()
        .filter(|t| !STOPWORDS.contains(t.as_str()))
        .cloned()
        .collect()
}

/// Stem tokens using Porter Stemmer
pub fn stem_tokens(tokens: &[String]) -> Vec<String> {
    // Protect against oversized token lists
    if tokens.len() > MAX_TOKENS {
        log::warn!("Token list too large for stemming: {}", tokens.len());
        return Vec::new();
    }

    let stemmer = create_stemmer();
    tokens
        .iter()
        .map(|t| {
            // Limit token size for stemming to prevent DoS
            if t.len() > 100 {
                t[0..100].to_string()
            } else {
                stemmer.stem(t).to_string()
            }
        })
        .collect()
}

/// Full preprocessing pipeline
pub fn preprocess(text: &str) -> PreprocessedText {
    // Validate input text
    if let Err(e) = validate_text_input(text) {
        log::warn!("Invalid input for preprocessing: {}", e);
        return PreprocessedText {
            original: String::new(),
            tokens: Vec::new(),
            stems: Vec::new(),
            normalized: String::new(),
        };
    }
    
    let original = text.to_string();
    
    // Sanitize input (control character removal)
    let sanitized = text
        .chars()
        .filter(|&c| !c.is_control() || c == '\n' || c == '\t')
        .collect::<String>();
    
    // Tokenize
    let all_tokens = tokenize(&sanitized);
    
    // Remove stopwords
    let tokens = remove_stopwords(&all_tokens);
    
    // Stem remaining tokens
    let stems = stem_tokens(&tokens);
    
    // Create normalized string for searching
    let normalized = stems.join(" ");
    
    PreprocessedText {
        original,
        tokens,
        stems,
        normalized,
    }
}

/// Preprocess text for storage in KB
/// Returns the normalized, stemmed version for indexing
pub fn preprocess_for_storage(text: &str) -> String {
    // Validate before processing
    if let Err(e) = validate_text_input(text) {
        log::warn!("Invalid input for storage preprocessing: {}", e);
        return String::new();
    }
    
    let result = preprocess(text);
    result.normalized
}

/// Preprocess query for searching
/// Same pipeline ensures query matches stored content
pub fn preprocess_query(query: &str) -> String {
    // Validate query with stricter limits for queries
    if query.len() > 4096 {  // 4KB max for queries
        log::warn!("Query too long for preprocessing: {} characters", query.len());
        return String::new();
    }
    
    preprocess_for_storage(query)
}

/// Safely clean text before processing
pub fn sanitize_text(text: &str) -> String {
    // Validate text length
    if text.len() > MAX_INPUT_LENGTH {
        return text[0..MAX_INPUT_LENGTH].to_string();
    }
    
    // Remove control characters except newlines and tabs
    text.chars()
        .filter(|&c| !c.is_control() || c == '\n' || c == '\t')
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize() {
        let tokens = tokenize("Hello, World! This is a test.");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
    }

    #[test]
    fn test_stopword_removal() {
        let tokens = vec!["the".to_string(), "quick".to_string(), "brown".to_string(), "fox".to_string()];
        let filtered = remove_stopwords(&tokens);
        assert!(!filtered.contains(&"the".to_string()));
        assert!(filtered.contains(&"quick".to_string()));
    }

    #[test]
    fn test_stemming() {
        let tokens = vec!["running".to_string(), "jumped".to_string(), "easier".to_string()];
        let stems = stem_tokens(&tokens);
        assert_eq!(stems[0], "run");
        assert_eq!(stems[1], "jump");
    }

    #[test]
    fn test_full_pipeline() {
        let result = preprocess("The quick brown foxes are running through the forest.");
        assert!(!result.tokens.contains(&"the".to_string()));
        assert!(result.stems.iter().any(|s| s.contains("fox")));
        assert!(!result.normalized.is_empty());
    }
}
