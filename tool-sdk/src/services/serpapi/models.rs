//! SerpAPI data models
//!
//! This module contains type definitions for SerpAPI requests and responses.

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Base search parameters common across all engines
pub trait SearchParams {
    /// Convert parameters to query parameters for the API request
    fn to_query_params(&self) -> HashMap<String, String>;
}

/// Google search parameters
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GoogleSearchParams {
    /// Search query
    pub q: String,
    
    /// Location
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    
    /// Google domain (com, co.uk, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_domain: Option<String>,
    
    /// Country code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gl: Option<String>,
    
    /// Language code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hl: Option<String>,
    
    /// Number of results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num: Option<u32>,
    
    /// Start offset
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<u32>,
    
    /// Safe search (active, off)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safe: Option<String>,
    
    /// Filter by time period
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tbs: Option<String>,
}

impl SearchParams for GoogleSearchParams {
    fn to_query_params(&self) -> HashMap<String, String> {
        let mut params = HashMap::new();
        
        params.insert("q".to_string(), self.q.clone());
        
        if let Some(ref location) = self.location {
            params.insert("location".to_string(), location.clone());
        }
        
        if let Some(ref google_domain) = self.google_domain {
            params.insert("google_domain".to_string(), google_domain.clone());
        }
        
        if let Some(ref gl) = self.gl {
            params.insert("gl".to_string(), gl.clone());
        }
        
        if let Some(ref hl) = self.hl {
            params.insert("hl".to_string(), hl.clone());
        }
        
        if let Some(num) = self.num {
            params.insert("num".to_string(), num.to_string());
        }
        
        if let Some(start) = self.start {
            params.insert("start".to_string(), start.to_string());
        }
        
        if let Some(ref safe) = self.safe {
            params.insert("safe".to_string(), safe.clone());
        }
        
        if let Some(ref tbs) = self.tbs {
            params.insert("tbs".to_string(), tbs.clone());
        }
        
        params
    }
}

/// Bing search parameters
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BingSearchParams {
    /// Search query
    pub q: String,
    
    /// Country code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc: Option<String>,
    
    /// Number of results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    
    /// First result position
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first: Option<u32>,
    
    /// Safe search setting (off, moderate, strict)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safe_search: Option<String>,
}

impl SearchParams for BingSearchParams {
    fn to_query_params(&self) -> HashMap<String, String> {
        let mut params = HashMap::new();
        
        params.insert("q".to_string(), self.q.clone());
        
        if let Some(ref cc) = self.cc {
            params.insert("cc".to_string(), cc.clone());
        }
        
        if let Some(count) = self.count {
            params.insert("count".to_string(), count.to_string());
        }
        
        if let Some(first) = self.first {
            params.insert("first".to_string(), first.to_string());
        }
        
        if let Some(ref safe_search) = self.safe_search {
            params.insert("safe_search".to_string(), safe_search.clone());
        }
        
        params
    }
}

/// Generic search response from SerpAPI
///
/// This is a simplified version. SerpAPI returns different structures
/// for different search engines, and the actual response is much more
/// complex. In a real implementation, you'd want to create more
/// detailed models for each search engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    /// Search metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_metadata: Option<SearchMetadata>,
    
    /// Search parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_parameters: Option<HashMap<String, String>>,
    
    /// Organic search results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organic_results: Option<Vec<OrganicResult>>,
    
    /// Related questions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_questions: Option<Vec<RelatedQuestion>>,
    
    /// Knowledge graph
    #[serde(skip_serializing_if = "Option::is_none")]
    pub knowledge_graph: Option<KnowledgeGraph>,
    
    /// Error message (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Search metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMetadata {
    /// Search ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    
    /// Status (success, error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    
    /// Request JSON endpoint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_endpoint: Option<String>,
    
    /// Request created timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    
    /// Request processed timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processed_at: Option<String>,
    
    /// Raw HTML file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_html_file: Option<String>,
    
    /// Total time taken in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_time_taken: Option<f64>,
}

/// Organic search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganicResult {
    /// Position in SERP
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<u32>,
    
    /// Result title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    
    /// Result link
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    
    /// Result display link
    #[serde(skip_serializing_if = "Option::is_none")]
    pub displayed_link: Option<String>,
    
    /// Result snippet
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
    
    /// Snippet highlighted words
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet_highlighted_words: Option<Vec<String>>,
    
    /// Whether the result is a featured snippet
    #[serde(skip_serializing_if = "Option::is_none")]
    pub featured_snippet: Option<bool>,
    
    /// Cached page URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_page_link: Option<String>,
    
    /// Related results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_pages_link: Option<String>,
    
    /// Site links
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sitelinks: Option<Vec<HashMap<String, String>>>,
}

/// Related question
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedQuestion {
    /// Question
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
    
    /// Answer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
    
    /// Source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<HashMap<String, String>>,
}

/// Knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    /// Title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    
    /// Type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_field: Option<String>,
    
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<HashMap<String, String>>,
}

/// Account response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountResponse {
    /// Account ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    
    /// Account email
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_email: Option<String>,
    
    /// API key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    
    /// Plan name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_name: Option<String>,
    
    /// Plan searches left
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_searches_left: Option<i32>,
    
    /// Plan searches per month
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_searches_per_month: Option<i32>,
    
    /// Plan expired
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_expired: Option<bool>,
    
    /// Plan expiration date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_expiration_date: Option<String>,
}

/// Search archive item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchArchiveItem {
    /// Search ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    
    /// Search status
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    
    /// Search engine
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_engine: Option<String>,
    
    /// Search query
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_query: Option<String>,
    
    /// Created at
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    
    /// Processed at
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processed_at: Option<String>,
    
    /// JSON endpoint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_endpoint: Option<String>,
}

/// Search archive response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchArchiveResponse {
    /// Searches
    #[serde(skip_serializing_if = "Option::is_none")]
    pub searches: Option<Vec<SearchArchiveItem>>,
}