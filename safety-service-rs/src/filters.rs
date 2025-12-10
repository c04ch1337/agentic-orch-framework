//! Content Filtering Module
//!
//! Implements comprehensive content filtering functionality including
//! classification, filtering strategies, and content sensitivity detection.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::Instant;
use async_trait::async_trait;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use regex::Regex;
use thiserror::Error;

/// Content filter error types
#[derive(Debug, Error)]
pub enum FilterError {
    #[error("Invalid filter configuration: {0}")]
    InvalidConfiguration(String),
    
    #[error("Unknown content type: {0}")]
    UnknownContentType(String),
    
    #[error("Unknown filter strategy: {0}")]
    UnknownFilterStrategy(String),
    
    #[error("Classification error: {0}")]
    ClassificationError(String),
    
    #[error("Filter application error: {0}")]
    FilterApplicationError(String),
    
    #[error("Content validation error: {0}")]
    ContentValidationError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Content type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContentType {
    /// Plain text content
    Text,
    /// HTML content
    Html,
    /// Command line instructions
    Command,
    /// Code snippets
    Code,
    /// User inputs
    UserInput,
    /// URLs and links
    Url,
    /// JSON data
    Json,
    /// XML data
    Xml,
    /// Image descriptions
    ImageDescription,
    /// Binary data
    Binary,
    /// Unknown content type
    Unknown,
}

impl std::fmt::Display for ContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text => write!(f, "text"),
            Self::Html => write!(f, "html"),
            Self::Command => write!(f, "command"),
            Self::Code => write!(f, "code"),
            Self::UserInput => write!(f, "user_input"),
            Self::Url => write!(f, "url"),
            Self::Json => write!(f, "json"),
            Self::Xml => write!(f, "xml"),
            Self::ImageDescription => write!(f, "image_description"),
            Self::Binary => write!(f, "binary"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

impl From<&str> for ContentType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "text" => Self::Text,
            "html" => Self::Html,
            "command" => Self::Command,
            "code" => Self::Code,
            "user_input" => Self::UserInput,
            "url" => Self::Url,
            "json" => Self::Json,
            "xml" => Self::Xml,
            "image_description" => Self::ImageDescription,
            "binary" => Self::Binary,
            _ => Self::Unknown,
        }
    }
}

/// Filter strategy enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FilterStrategy {
    /// Remove harmful content
    Remove,
    /// Replace harmful content with placeholders
    Replace,
    /// Block content completely
    Block,
    /// Allow but mark content as potentially harmful
    Mark,
    /// Sanitize content
    Sanitize,
    /// Allow content without filtering
    Allow,
}

impl std::fmt::Display for FilterStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Remove => write!(f, "remove"),
            Self::Replace => write!(f, "replace"),
            Self::Block => write!(f, "block"),
            Self::Mark => write!(f, "mark"),
            Self::Sanitize => write!(f, "sanitize"),
            Self::Allow => write!(f, "allow"),
        }
    }
}

impl From<&str> for FilterStrategy {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "remove" => Self::Remove,
            "replace" => Self::Replace,
            "block" => Self::Block,
            "mark" => Self::Mark,
            "sanitize" => Self::Sanitize,
            "allow" => Self::Allow,
            _ => Self::Block, // Default to most restrictive
        }
    }
}

/// Content classification category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContentCategory {
    /// Safe content
    Safe,
    /// Potentially harmful content
    PotentiallyHarmful,
    /// Harmful content
    Harmful,
    /// Malicious content
    Malicious,
    /// Explicit content
    Explicit,
    /// Content with privacy concerns
    PrivacyConcern,
    /// Content with security concerns
    SecurityConcern,
    /// System command execution
    SystemCommand,
    /// Code execution
    CodeExecution,
    /// Uncategorized content
    Uncategorized,
}

impl std::fmt::Display for ContentCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Safe => write!(f, "safe"),
            Self::PotentiallyHarmful => write!(f, "potentially_harmful"),
            Self::Harmful => write!(f, "harmful"),
            Self::Malicious => write!(f, "malicious"),
            Self::Explicit => write!(f, "explicit"),
            Self::PrivacyConcern => write!(f, "privacy_concern"),
            Self::SecurityConcern => write!(f, "security_concern"),
            Self::SystemCommand => write!(f, "system_command"),
            Self::CodeExecution => write!(f, "code_execution"),
            Self::Uncategorized => write!(f, "uncategorized"),
        }
    }
}

/// Classification result for content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    /// Map of categories to confidence scores (0.0-1.0)
    pub categories: HashMap<ContentCategory, f32>,
    /// Primary content category (highest confidence)
    pub primary_category: ContentCategory,
    /// Content type
    pub content_type: ContentType,
    /// Classification timestamp
    pub timestamp: u64,
    /// Metadata and additional properties
    pub metadata: HashMap<String, String>,
}

impl ClassificationResult {
    /// Create a new classification result
    pub fn new(
        categories: HashMap<ContentCategory, f32>,
        content_type: ContentType,
    ) -> Self {
        // Find the primary category (highest confidence)
        let (primary_category, _) = categories
            .iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .unwrap_or((&ContentCategory::Uncategorized, &0.0));
            
        Self {
            categories,
            primary_category: *primary_category,
            content_type,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            metadata: HashMap::new(),
        }
    }
    
    /// Check if the content is considered safe
    pub fn is_safe(&self) -> bool {
        matches!(self.primary_category, ContentCategory::Safe)
    }
    
    /// Get the confidence score for a specific category
    pub fn confidence(&self, category: ContentCategory) -> f32 {
        *self.categories.get(&category).unwrap_or(&0.0)
    }
    
    /// Add metadata to the classification result
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

/// Filter rules configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterRules {
    /// Threshold for overall content filtering (0.0-1.0)
    pub threshold: f32,
    /// Category-specific thresholds
    pub category_thresholds: HashMap<ContentCategory, f32>,
    /// Default strategy for content that exceeds thresholds
    pub default_strategy: FilterStrategy,
    /// Category-specific strategies
    pub category_strategies: HashMap<ContentCategory, FilterStrategy>,
    /// Content type specific strategies
    pub content_type_strategies: HashMap<ContentType, FilterStrategy>,
    /// List of patterns to always block (regex)
    pub block_patterns: Vec<String>,
    /// List of patterns to always allow (regex)
    pub allow_patterns: Vec<String>,
    /// Additional rule parameters
    pub parameters: HashMap<String, String>,
}

impl Default for FilterRules {
    fn default() -> Self {
        let mut category_thresholds = HashMap::new();
        category_thresholds.insert(ContentCategory::Safe, 0.0);
        category_thresholds.insert(ContentCategory::PotentiallyHarmful, 0.5);
        category_thresholds.insert(ContentCategory::Harmful, 0.7);
        category_thresholds.insert(ContentCategory::Malicious, 0.8);
        category_thresholds.insert(ContentCategory::Explicit, 0.7);
        category_thresholds.insert(ContentCategory::PrivacyConcern, 0.6);
        category_thresholds.insert(ContentCategory::SecurityConcern, 0.7);
        category_thresholds.insert(ContentCategory::SystemCommand, 0.5);
        category_thresholds.insert(ContentCategory::CodeExecution, 0.5);
        
        let mut category_strategies = HashMap::new();
        category_strategies.insert(ContentCategory::Safe, FilterStrategy::Allow);
        category_strategies.insert(ContentCategory::PotentiallyHarmful, FilterStrategy::Mark);
        category_strategies.insert(ContentCategory::Harmful, FilterStrategy::Replace);
        category_strategies.insert(ContentCategory::Malicious, FilterStrategy::Block);
        category_strategies.insert(ContentCategory::Explicit, FilterStrategy::Replace);
        category_strategies.insert(ContentCategory::PrivacyConcern, FilterStrategy::Sanitize);
        category_strategies.insert(ContentCategory::SecurityConcern, FilterStrategy::Block);
        category_strategies.insert(ContentCategory::SystemCommand, FilterStrategy::Mark);
        category_strategies.insert(ContentCategory::CodeExecution, FilterStrategy::Mark);
        
        let mut content_type_strategies = HashMap::new();
        content_type_strategies.insert(ContentType::Text, FilterStrategy::Mark);
        content_type_strategies.insert(ContentType::Html, FilterStrategy::Sanitize);
        content_type_strategies.insert(ContentType::Command, FilterStrategy::Mark);
        content_type_strategies.insert(ContentType::Code, FilterStrategy::Mark);
        content_type_strategies.insert(ContentType::UserInput, FilterStrategy::Sanitize);
        content_type_strategies.insert(ContentType::Url, FilterStrategy::Sanitize);
        content_type_strategies.insert(ContentType::Json, FilterStrategy::Sanitize);
        content_type_strategies.insert(ContentType::Xml, FilterStrategy::Sanitize);
        content_type_strategies.insert(ContentType::ImageDescription, FilterStrategy::Mark);
        content_type_strategies.insert(ContentType::Binary, FilterStrategy::Block);
        
        Self {
            threshold: 0.7,
            category_thresholds,
            default_strategy: FilterStrategy::Block,
            category_strategies,
            content_type_strategies,
            block_patterns: vec![],
            allow_patterns: vec![],
            parameters: HashMap::new(),
        }
    }
}

/// Filter result containing the filtered content and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterResult {
    /// Original content
    pub original_content: String,
    /// Filtered content
    pub filtered_content: String,
    /// Whether content was modified
    pub modified: bool,
    /// Strategy applied
    pub strategy_applied: FilterStrategy,
    /// Classification result
    pub classification: ClassificationResult,
    /// Filter execution time in milliseconds
    pub execution_time_ms: u64,
    /// Explanation for filtering decision
    pub explanation: String,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl FilterResult {
    /// Create a new filter result
    pub fn new(
        original_content: String,
        filtered_content: String,
        modified: bool,
        strategy_applied: FilterStrategy,
        classification: ClassificationResult,
        execution_time_ms: u64,
        explanation: String,
    ) -> Self {
        Self {
            original_content,
            filtered_content,
            modified,
            strategy_applied,
            classification,
            execution_time_ms,
            explanation,
            metadata: HashMap::new(),
        }
    }
    
    /// Check if content was blocked
    pub fn is_blocked(&self) -> bool {
        self.strategy_applied == FilterStrategy::Block
    }
    
    /// Add metadata to the filter result
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

/// Content classifier trait
#[async_trait]
pub trait ContentClassifier: Send + Sync {
    /// Classify content
    async fn classify(&self, content: &str, content_type: ContentType) -> Result<ClassificationResult, FilterError>;
    
    /// Get the classifier name
    fn name(&self) -> &str;
}

/// Rule-based classifier
pub struct RuleBasedClassifier {
    /// Patterns for each category
    patterns: HashMap<ContentCategory, Vec<Regex>>,
    /// Category weights
    weights: HashMap<ContentCategory, f32>,
    /// Normalized weights for final score calculation
    normalized_weights: HashMap<ContentCategory, f32>,
}

impl RuleBasedClassifier {
    /// Create a new rule-based classifier
    pub fn new() -> Result<Self, FilterError> {
        let mut patterns = HashMap::new();
        let mut weights = HashMap::new();
        
        // Add patterns for each category
        patterns.insert(ContentCategory::PotentiallyHarmful, vec![
            Regex::new(r"(?i)\b(hack|crack|exploit)\b").map_err(|e| FilterError::InvalidConfiguration(e.to_string()))?,
        ]);
        
        patterns.insert(ContentCategory::Harmful, vec![
            Regex::new(r"(?i)\b(attack|vulnerability|inject|exploit|overflow)\b").map_err(|e| FilterError::InvalidConfiguration(e.to_string()))?,
        ]);
        
        patterns.insert(ContentCategory::Malicious, vec![
            Regex::new(r"(?i)\b(malware|virus|trojan|ransomware|rootkit)\b").map_err(|e| FilterError::InvalidConfiguration(e.to_string()))?,
        ]);
        
        patterns.insert(ContentCategory::Explicit, vec![
            Regex::new(r"(?i)\b(explicit|nsfw|pornography|xxx)\b").map_err(|e| FilterError::InvalidConfiguration(e.to_string()))?,
        ]);
        
        patterns.insert(ContentCategory::PrivacyConcern, vec![
            Regex::new(r"(?i)\b(password|credential|token|key|secret|ssn|social security)\b").map_err(|e| FilterError::InvalidConfiguration(e.to_string()))?,
        ]);
        
        patterns.insert(ContentCategory::SecurityConcern, vec![
            Regex::new(r"(?i)\b(sql\s*injection|xss|csrf|cross\s*site|remote\s*code\s*execution|rce)\b").map_err(|e| FilterError::InvalidConfiguration(e.to_string()))?,
        ]);
        
        patterns.insert(ContentCategory::SystemCommand, vec![
            Regex::new(r"(?i)\b(sudo|rm\s+-rf|format|mkfs|fdisk|dd\s+if|chmod\s+777)\b").map_err(|e| FilterError::InvalidConfiguration(e.to_string()))?,
        ]);
        
        patterns.insert(ContentCategory::CodeExecution, vec![
            Regex::new(r"(?i)\b(eval\s*\(|exec\s*\(|system\s*\(|shell_exec\s*\(|subprocess\.call|os\.system)\b").map_err(|e| FilterError::InvalidConfiguration(e.to_string()))?,
        ]);
        
        // Set category weights
        weights.insert(ContentCategory::PotentiallyHarmful, 0.5);
        weights.insert(ContentCategory::Harmful, 0.7);
        weights.insert(ContentCategory::Malicious, 0.9);
        weights.insert(ContentCategory::Explicit, 0.8);
        weights.insert(ContentCategory::PrivacyConcern, 0.6);
        weights.insert(ContentCategory::SecurityConcern, 0.8);
        weights.insert(ContentCategory::SystemCommand, 0.7);
        weights.insert(ContentCategory::CodeExecution, 0.7);
        
        // Calculate normalized weights
        let total_weight: f32 = weights.values().sum();
        let mut normalized_weights = HashMap::new();
        
        for (category, weight) in &weights {
            normalized_weights.insert(*category, weight / total_weight);
        }
        
        Ok(Self {
            patterns,
            weights,
            normalized_weights,
        })
    }
    
    /// Add a pattern for a category
    pub fn add_pattern(&mut self, category: ContentCategory, pattern: &str) -> Result<(), FilterError> {
        let regex = Regex::new(pattern)
            .map_err(|e| FilterError::InvalidConfiguration(format!("Invalid regex pattern: {}", e)))?;
            
        self.patterns.entry(category).or_insert_with(Vec::new).push(regex);
        Ok(())
    }
    
    /// Set weight for a category
    pub fn set_weight(&mut self, category: ContentCategory, weight: f32) {
        self.weights.insert(category, weight);
        
        // Recalculate normalized weights
        let total_weight: f32 = self.weights.values().sum();
        
        for (cat, weight) in &self.weights {
            self.normalized_weights.insert(*cat, weight / total_weight);
        }
    }
    
    /// Calculate score for a category based on pattern matches
    fn calculate_score(&self, content: &str, category: ContentCategory) -> f32 {
        if let Some(patterns) = self.patterns.get(&category) {
            let mut matches = 0;
            
            for pattern in patterns {
                if pattern.is_match(content) {
                    matches += 1;
                }
            }
            
            if patterns.is_empty() {
                0.0
            } else {
                matches as f32 / patterns.len() as f32
            }
        } else {
            0.0
        }
    }
}

#[async_trait]
impl ContentClassifier for RuleBasedClassifier {
    async fn classify(&self, content: &str, content_type: ContentType) -> Result<ClassificationResult, FilterError> {
        let mut categories = HashMap::new();
        
        // Calculate scores for each category
        for category in &[
            ContentCategory::PotentiallyHarmful,
            ContentCategory::Harmful,
            ContentCategory::Malicious,
            ContentCategory::Explicit,
            ContentCategory::PrivacyConcern,
            ContentCategory::SecurityConcern,
            ContentCategory::SystemCommand,
            ContentCategory::CodeExecution,
        ] {
            let score = self.calculate_score(content, *category);
            if score > 0.0 {
                categories.insert(*category, score);
            }
        }
        
        // If no categories matched, mark as safe
        if categories.is_empty() {
            categories.insert(ContentCategory::Safe, 1.0);
        } else {
            // Add safe category with inverse score
            let max_score = categories.values().cloned().fold(0.0, f32::max);
            categories.insert(ContentCategory::Safe, 1.0 - max_score);
        }
        
        Ok(ClassificationResult::new(categories, content_type))
    }
    
    fn name(&self) -> &str {
        "RuleBasedClassifier"
    }
}

/// Combined classifier that aggregates results from multiple classifiers
pub struct CombinedClassifier {
    /// List of classifiers
    classifiers: Vec<Box<dyn ContentClassifier>>,
    /// Weights for each classifier
    weights: Vec<f32>,
}

impl CombinedClassifier {
    /// Create a new combined classifier
    pub fn new() -> Self {
        Self {
            classifiers: Vec::new(),
            weights: Vec::new(),
        }
    }
    
    /// Add a classifier with weight
    pub fn add_classifier(&mut self, classifier: Box<dyn ContentClassifier>, weight: f32) {
        self.classifiers.push(classifier);
        self.weights.push(weight);
    }
}

#[async_trait]
impl ContentClassifier for CombinedClassifier {
    async fn classify(&self, content: &str, content_type: ContentType) -> Result<ClassificationResult, FilterError> {
        if self.classifiers.is_empty() {
            return Err(FilterError::ClassificationError("No classifiers available".to_string()));
        }
        
        let mut combined_categories = HashMap::new();
        let total_weight: f32 = self.weights.iter().sum();
        
        for (classifier, weight) in self.classifiers.iter().zip(self.weights.iter()) {
            let result = classifier.classify(content, content_type).await?;
            let normalized_weight = weight / total_weight;
            
            for (category, score) in result.categories {
                let weighted_score = score * normalized_weight;
                *combined_categories.entry(category).or_insert(0.0) += weighted_score;
            }
        }
        
        Ok(ClassificationResult::new(combined_categories, content_type))
    }
    
    fn name(&self) -> &str {
        "CombinedClassifier"
    }
}

/// Content filter implementation
pub struct ContentFilter {
    /// Classifier for content
    classifier: Box<dyn ContentClassifier>,
    /// Filter rules
    rules: RwLock<FilterRules>,
    /// Block patterns compiled to regex
    block_patterns: RwLock<Vec<Regex>>,
    /// Allow patterns compiled to regex
    allow_patterns: RwLock<Vec<Regex>>,
}

impl ContentFilter {
    /// Create a new content filter with the given classifier and rules
    pub fn new(classifier: Box<dyn ContentClassifier>, rules: FilterRules) -> Result<Self, FilterError> {
        // Compile block and allow patterns
        let mut block_patterns = Vec::new();
        let mut allow_patterns = Vec::new();
        
        for pattern in &rules.block_patterns {
            let regex = Regex::new(pattern)
                .map_err(|e| FilterError::InvalidConfiguration(format!("Invalid block pattern: {}", e)))?;
            block_patterns.push(regex);
        }
        
        for pattern in &rules.allow_patterns {
            let regex = Regex::new(pattern)
                .map_err(|e| FilterError::InvalidConfiguration(format!("Invalid allow pattern: {}", e)))?;
            allow_patterns.push(regex);
        }
        
        Ok(Self {
            classifier,
            rules: RwLock::new(rules),
            block_patterns: RwLock::new(block_patterns),
            allow_patterns: RwLock::new(allow_patterns),
        })
    }
    
    /// Create a default content filter
    pub fn default_filter() -> Result<Self, FilterError> {
        let classifier = Box::new(RuleBasedClassifier::new()?);
        let rules = FilterRules::default();
        
        Self::new(classifier, rules)
    }
    
    /// Update filter rules
    pub fn update_rules(&self, new_rules: FilterRules) -> Result<(), FilterError> {
        // Compile new block and allow patterns
        let mut block_patterns = Vec::new();
        for pattern in &new_rules.block_patterns {
            let regex = Regex::new(pattern)
                .map_err(|e| FilterError::InvalidConfiguration(format!("Invalid block pattern: {}", e)))?;
            block_patterns.push(regex);
        }
        
        let mut allow_patterns = Vec::new();
        for pattern in &new_rules.allow_patterns {
            let regex = Regex::new(pattern)
                .map_err(|e| FilterError::InvalidConfiguration(format!("Invalid allow pattern: {}", e)))?;
            allow_patterns.push(regex);
        }
        
        // Update patterns and rules
        {
            let mut block = self.block_patterns.write().unwrap();
            *block = block_patterns;
        }
        
        {
            let mut allow = self.allow_patterns.write().unwrap();
            *allow = allow_patterns;
        }
        
        {
            let mut rules = self.rules.write().unwrap();
            *rules = new_rules;
        }
        
        Ok(())
    }
    
    /// Filter content
    pub async fn filter_content(
        &self,
        content: &str,
        content_type: ContentType,
    ) -> Result<FilterResult, FilterError> {
        let start_time = Instant::now();
        
        // Check allow patterns first
        {
            let allow_patterns = self.allow_patterns.read().unwrap();
            for pattern in allow_patterns.iter() {
                if pattern.is_match(content) {
                    let classification = self.classifier.classify(content, content_type).await?;
                    
                    return Ok(FilterResult::new(
                        content.to_string(),
                        content.to_string(),
                        false,
                        FilterStrategy::Allow,
                        classification,
                        start_time.elapsed().as_millis() as u64,
                        "Content explicitly allowed by pattern".to_string(),
                    ));
                }
            }
        }
        
        // Check block patterns
        {
            let block_patterns = self.block_patterns.read().unwrap();
            for pattern in block_patterns.iter() {
                if pattern.is_match(content) {
                    let classification = self.classifier.classify(content, content_type).await?;
                    
                    return Ok(FilterResult::new(
                        content.to_string(),
                        "".to_string(),
                        true,
                        FilterStrategy::Block,
                        classification,
                        start_time.elapsed().as_millis() as u64,
                        "Content explicitly blocked by pattern".to_string(),
                    ));
                }
            }
        }
        
        // Classify the content
        let classification = self.classifier.classify(content, content_type).await?;
        
        // Determine filtering strategy
        let (strategy, explanation) = self.determine_strategy(&classification)?;
        
        // Apply the filtering strategy
        let (filtered_content, modified) = self.apply_strategy(content, &strategy, &classification)?;
        
        Ok(FilterResult::new(
            content.to_string(),
            filtered_content,
            modified,
            strategy,
            classification,
            start_time.elapsed().as_millis() as u64,
            explanation,
        ))
    }
    
    /// Determine filtering strategy based on classification
    fn determine_strategy(&self, classification: &ClassificationResult) -> Result<(FilterStrategy, String), FilterError> {
        let rules = self.rules.read().unwrap();
        
        // Check category thresholds
        for (category, threshold) in &rules.category_thresholds {
            let score = classification.confidence(*category);
            
            if score >= *threshold {
                // If this category exceeds threshold, get the strategy
                if let Some(strategy) = rules.category_strategies.get(category) {
                    let explanation = format!(
                        "Category {} exceeded threshold {:.2} with score {:.2}",
                        category, threshold, score
                    );
                    return Ok((*strategy, explanation));
                }
            }
        }
        
        // Check content type strategies
        if let Some(strategy) = rules.content_type_strategies.get(&classification.content_type) {
            let explanation = format!(
                "Applied content type strategy for {}",
                classification.content_type
            );
            return Ok((*strategy, explanation));
        }
        
        // Use default strategy
        Ok((
            rules.default_strategy,
            "Applied default strategy".to_string(),
        ))
    }
    
    /// Apply filtering strategy to content
    fn apply_strategy(
        &self,
        content: &str,
        strategy: &FilterStrategy,
        classification: &ClassificationResult,
    ) -> Result<(String, bool), FilterError> {
        match strategy {
            FilterStrategy::Allow => Ok((content.to_string(), false)),
            
            FilterStrategy::Block => Ok(("".to_string(), true)),
            
            FilterStrategy::Mark => {
                let marked = format!(
                    "[POTENTIALLY SENSITIVE CONTENT - Category: {}] {}",
                    classification.primary_category, content
                );
                Ok((marked, true))
            },
            
            FilterStrategy::Remove => {
                // For simplicity, we'll remove the entire content if it's harmful
                // A more sophisticated implementation would selectively remove harmful sections
                match classification.primary_category {
                    ContentCategory::Safe | ContentCategory::Uncategorized => {
                        Ok((content.to_string(), false))
                    },
                    _ => {
                        Ok(("".to_string(), true))
                    },
                }
            },
            
            FilterStrategy::Replace => {
                // For simplicity, we'll replace the entire content with placeholders
                // A more sophisticated implementation would selectively replace harmful sections
                match classification.primary_category {
                    ContentCategory::Safe | ContentCategory::Uncategorized => {
                        Ok((content.to_string(), false))
                    },
                    _ => {
                        Ok(("[CONTENT REMOVED DUE TO SAFETY CONCERNS]".to_string(), true))
                    },
                }
            },
            
            FilterStrategy::Sanitize => {
                // Perform content type specific sanitization
                match classification.content_type {
                    ContentType::Html => {
                        let sanitized = self.sanitize_html(content)?;
                        Ok((sanitized, content != sanitized))
                    },
                    ContentType::Command => {
                        let sanitized = self.sanitize_command(content)?;
                        Ok((sanitized, content != sanitized))
                    },
                    ContentType::Url => {
                        let sanitized = self.sanitize_url(content)?;
                        Ok((sanitized, content != sanitized))
                    },
                    _ => {
                        // Default sanitization removes special characters
                        let sanitized = content.chars()
                            .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '.' || *c == ',' || *c == '!' || *c == '?')
                            .collect::<String>();
                        Ok((sanitized, content != sanitized))
                    },
                }
            },
        }
    }
    
    /// Sanitize HTML content
    fn sanitize_html(&self, content: &str) -> Result<String, FilterError> {
        // This is a simplified implementation
        // A real implementation would use a proper HTML sanitizer
        // like ammonia or html5ever
        
        let sanitized = content
            .replace("<script", "&lt;script")
            .replace("</script>", "&lt;/script&gt;")
            .replace("javascript:", "disabled-javascript:")
            .replace("onerror=", "disabled-onerror=")
            .replace("onclick=", "disabled-onclick=")
            .replace("<iframe", "&lt;iframe")
            .replace("</iframe>", "&lt;/iframe&gt;");
            
        Ok(sanitized)
    }
    
    /// Sanitize command content
    fn sanitize_command(&self, content: &str) -> Result<String, FilterError> {
        // This is a simplified implementation
        // A real implementation would use more sophisticated command sanitization
        
        let sanitized = content
            .replace(";", "")
            .replace("|", "")
            .replace("&&", "")
            .replace("||", "")
            .replace(">", "")
            .replace("<", "")
            .replace("$", "")
            .replace("`", "")
            .replace("(", "")
            .replace(")", "");
            
        Ok(sanitized)
    }
    
    /// Sanitize URL content
    fn sanitize_url(&self, content: &str) -> Result<String, FilterError> {
        // This is a simplified implementation
        // A real implementation would parse and validate the URL properly
        
        if content.starts_with("http://") || content.starts_with("https://") {
            Ok(content.to_string())
        } else {
            Ok(format!("https://{}", content))
        }
    }
}

/// Factory for creating and configuring different types of filters
pub struct FilterFactory;

impl FilterFactory {
    /// Create a basic rule-based filter
    pub fn create_rule_based_filter() -> Result<ContentFilter, FilterError> {
        let classifier = Box::new(RuleBasedClassifier::new()?);
        let rules = FilterRules::default();
        
        ContentFilter::new(classifier, rules)
    }
    
    /// Create a combined classifier filter
    pub fn create_combined_filter() -> Result<ContentFilter, FilterError> {
        let rule_based = RuleBasedClassifier::new()?;
        
        let mut combined = CombinedClassifier::new();
        combined.add_classifier(Box::new(rule_based), 1.0);
        
        let rules = FilterRules::default();
        
        ContentFilter::new(Box::new(combined), rules)
    }
    
    /// Create a strict security filter
    pub fn create_strict_security_filter() -> Result<ContentFilter, FilterError> {
        let mut classifier = RuleBasedClassifier::new()?;
        
        // Add additional security patterns
        classifier.add_pattern(ContentCategory::SecurityConcern, r"(?i)\b(password|token|secret|key|credential)\b")?;
        classifier.add_pattern(ContentCategory::SecurityConcern, r"(?i)\b(auth|login|authentication)\b")?;
        
        // Increase weights for security categories
        classifier.set_weight(ContentCategory::SecurityConcern, 1.0);
        classifier.set_weight(ContentCategory::SystemCommand, 0.9);
        classifier.set_weight(ContentCategory::CodeExecution, 0.9);
        
        // Customize rules
        let mut rules = FilterRules::default();
        rules.threshold = 0.5; // Lower threshold for stricter filtering
        
        // Set strict strategies for security categories
        rules.category_strategies.insert(ContentCategory::SecurityConcern, FilterStrategy::Block);
        rules.category_strategies.insert(ContentCategory::SystemCommand, FilterStrategy::Block);
        rules.category_strategies.insert(ContentCategory::CodeExecution, FilterStrategy::Block);
        
        ContentFilter::new(Box::new(classifier), rules)
    }
}

/// FilterStrategy execution statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FilterStats {
    /// Total number of filter operations
    pub total_operations: usize,
    /// Number of allowed content
    pub allowed_count: usize,
    /// Number of blocked content
    pub blocked_count: usize,
    /// Number of modified content
    pub modified_count: usize,
    /// Counts by content category
    pub category_counts: HashMap<ContentCategory, usize>,
    /// Counts by content type
    pub content_type_counts: HashMap<ContentType, usize>,
    /// Average execution time in milliseconds
    pub avg_execution_time_ms: f64,
}

impl FilterStats {
    /// Create a new empty stats object
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Update stats with a filter result
    pub fn update(&mut self, result: &FilterResult) {
        self.total_operations += 1;
        
        if result.is_blocked() {
            self.blocked_count += 1;
        } else if result.modified {
            self.modified_count += 1;
        } else {
            self.allowed_count += 1;
        }
        
        *self.category_counts.entry(result.classification.primary_category).or_insert(0) += 1;
        *self.content_type_counts.entry(result.classification.content_type).or_insert(0) += 1;
        
        // Update average execution time
        let total_time = self.avg_execution_time_ms * (self.total_operations - 1) as f64;
        self.avg_execution_time_ms = (total_time + result.execution_time_ms as f64) / self.total_operations as f64;
    }
    
    /// Reset stats
    pub fn reset(&mut self) {
        self.total_operations = 0;
        self.allowed_count = 0;
        self.blocked_count = 0;
        self.modified_count = 0;
        self.category_counts.clear();
        self.content_type_counts.clear();
        self.avg_execution_time_ms = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_rule_based_classifier() {
        let classifier = RuleBasedClassifier::new().unwrap();
        
        // Test safe content
        let result = classifier.classify("Hello world, this is a safe message.", ContentType::Text).await.unwrap();
        assert_eq!(result.primary_category, ContentCategory::Safe);
        
        // Test harmful content
        let result = classifier.classify("This contains a vulnerability that can be exploited.", ContentType::Text).await.unwrap();
        assert_ne!(result.primary_category, ContentCategory::Safe);
        assert!(result.confidence(ContentCategory::Harmful) > 0.0);
    }
    
    #[tokio::test]
    async fn test_content_filter() {
        let filter = ContentFilter::default_filter().unwrap();
        
        // Test safe content
        let result = filter.filter_content("Hello world, this is a safe message.", ContentType::Text).await.unwrap();
        assert!(!result.modified);
        assert_eq!(result.strategy_applied, FilterStrategy::Allow);
        
        // Test harmful content
        let result = filter.filter_content("This is malware that contains a vulnerability for exploitation.", ContentType::Text).await.unwrap();
        assert!(result.modified);
    }
    
    #[tokio::test]
    async fn test_filter_strategies() {
        let filter = ContentFilter::default_filter().unwrap();
        
        // Test block strategy
        let content = "This is malware that contains a rootkit";
        let classification = filter.classifier.classify(content, ContentType::Text).await.unwrap();
        let (filtered, modified) = filter.apply_strategy(content, &FilterStrategy::Block, &classification).unwrap();
        assert!(modified);
        assert!(filtered.is_empty());
        
        // Test mark strategy
        let (filtered, modified) = filter.apply_strategy(content, &FilterStrategy::Mark, &classification).unwrap();
        assert!(modified);
        assert!(filtered.contains("[POTENTIALLY SENSITIVE CONTENT"));
        
        // Test replace strategy
        let (filtered, modified) = filter.apply_strategy(content, &FilterStrategy::Replace, &classification).unwrap();
        assert!(modified);
        assert!(filtered.contains("[CONTENT REMOVED"));
    }
    
    #[tokio::test]
    async fn test_html_sanitization() {
        let filter = ContentFilter::default_filter().unwrap();
        
        let html = "<script>alert('XSS')</script><div>Hello</div>";
        let sanitized = filter.sanitize_html(html).unwrap();
        
        assert!(!sanitized.contains("<script>"));
        assert!(sanitized.contains("&lt;script"));
        assert!(sanitized.contains("<div>"));
    }
    
    #[tokio::test]
    async fn test_command_sanitization() {
        let filter = ContentFilter::default_filter().unwrap();
        
        let command = "ls -la; rm -rf /";
        let sanitized = filter.sanitize_command(command).unwrap();
        
        assert!(!sanitized.contains(";"));
        assert!(sanitized.contains("ls -la"));
    }
    
    #[tokio::test]
    async fn test_filter_stats() {
        let mut stats = FilterStats::new();
        
        // Create sample filter results
        let allowed_result = FilterResult::new(
            "Hello world".to_string(),
            "Hello world".to_string(),
            false,
            FilterStrategy::Allow,
            ClassificationResult::new(
                [(ContentCategory::Safe, 1.0)].into_iter().collect(),
                ContentType::Text,
            ),
            10,
            "Content allowed".to_string(),
        );
        
        let blocked_result = FilterResult::new(
            "Malware detected".to_string(),
            "".to_string(),
            true,
            FilterStrategy::Block,
            ClassificationResult::new(
                [(ContentCategory::Malicious, 1.0)].into_iter().collect(),
                ContentType::Text,
            ),
            15,
            "Content blocked".to_string(),
        );
        
        // Update stats
        stats.update(&allowed_result);
        stats.update(&blocked_result);
        
        assert_eq!(stats.total_operations, 2);
        assert_eq!(stats.allowed_count, 1);
        assert_eq!(stats.blocked_count, 1);
        assert_eq!(stats.category_counts.get(&ContentCategory::Safe).unwrap(), &1);
        assert_eq!(stats.category_counts.get(&ContentCategory::Malicious).unwrap(), &1);
        assert_eq!(stats.content_type_counts.get(&ContentType::Text).unwrap(), &2);
        assert!((stats.avg_execution_time_ms - 12.5).abs() < 0.001);
        
        // Test reset
        stats.reset();
        assert_eq!(stats.total_operations, 0);
    }
}