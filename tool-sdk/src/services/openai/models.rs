//! OpenAI API data models
//!
//! This module contains type definitions for OpenAI API requests and responses.

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Chat message role
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System message
    System,
    /// User message
    User,
    /// Assistant message
    Assistant,
    /// Function message
    Function,
}

impl ToString for Role {
    fn to_string(&self) -> String {
        match self {
            Role::System => "system".to_string(),
            Role::User => "user".to_string(),
            Role::Assistant => "assistant".to_string(),
            Role::Function => "function".to_string(),
        }
    }
}

/// A chat message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// The role of the message author
    pub role: String,
    
    /// The content of the message
    pub content: String,
    
    /// Optional name of the author
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Chat completion request
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatCompletionRequest {
    /// ID of the model to use
    pub model: String,
    
    /// The messages to generate chat completions for
    pub messages: Vec<ChatMessage>,
    
    /// Sampling temperature (0.0-2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    
    /// Top-p sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    
    /// Number of chat completions to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,
    
    /// Whether to stream responses
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    
    /// Stop sequences that end generation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    
    /// Maximum number of tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    
    /// Presence penalty
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    
    /// Frequency penalty
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    
    /// Logit bias
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<HashMap<String, f32>>,
    
    /// User identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

/// A chat completion choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChoice {
    /// Index of the choice
    pub index: u32,
    
    /// The generated message
    pub message: ChatCompletionMessage,
    
    /// Reason for finishing
    pub finish_reason: Option<String>,
}

/// A message in a chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionMessage {
    /// Role of the message
    pub role: String,
    
    /// Content of the message
    pub content: Option<String>,
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    /// Number of prompt tokens
    pub prompt_tokens: u32,
    
    /// Number of completion tokens
    pub completion_tokens: u32,
    
    /// Total tokens used
    pub total_tokens: u32,
}

/// Chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    /// Response ID
    pub id: String,
    
    /// Object type
    pub object: String,
    
    /// Creation timestamp
    pub created: u64,
    
    /// Model used
    pub model: String,
    
    /// Choices generated
    pub choices: Vec<ChatCompletionChoice>,
    
    /// Token usage statistics
    pub usage: Usage,
}

/// Embedding input type that can be either a string or array of strings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EmbeddingInput {
    /// A single string input
    String(String),
    
    /// An array of string inputs
    Array(Vec<String>),
}

/// Embedding request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    /// ID of the model to use
    pub model: String,
    
    /// Input text to embed
    pub input: EmbeddingInput,
    
    /// The format to return the embeddings in
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding_format: Option<String>,
    
    /// User identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

/// A single embedding result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    /// The embedding vector
    pub embedding: Vec<f32>,
    
    /// Index in the input array
    pub index: u32,
}

/// Embedding response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    /// Response ID
    pub id: String,
    
    /// Object type
    pub object: String,
    
    /// Array of embeddings
    pub data: Vec<Embedding>,
    
    /// Model used
    pub model: String,
    
    /// Token usage statistics
    pub usage: Usage,
}

/// Model details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    /// Model ID
    pub id: String,
    
    /// Object type
    pub object: String,
    
    /// Creation timestamp
    pub created: u64,
    
    /// Model owner
    pub owned_by: String,
}

/// List models response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListModelsResponse {
    /// Object type
    pub object: String,
    
    /// Array of models
    pub data: Vec<Model>,
}