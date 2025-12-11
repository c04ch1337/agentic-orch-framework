// llm-service-rs/src/llm_client.rs
//
// HTTP Client for interacting with LLM providers (OpenAI-compatible API)
//
// This module provides:
// - Real HTTP calls to LLM API providers via reqwest
// - Exponential backoff retry mechanism for resilient operation
// - Proper error handling with classification of retryable vs. non-retryable errors
// - Configuration via environment variables
//
// Configuration (.env file):
// - LLM_API_KEY: API key for the LLM provider
// - LLM_API_URL: API endpoint URL (defaults to OpenAI compatible endpoint)
// - LLM_MODEL: Model to use (e.g. "gpt-3.5-turbo", "anthropic/claude-3.5-sonnet")
// - LLM_MAX_RETRIES: Maximum number of retry attempts (default: 3)
// - LLM_INITIAL_RETRY_DELAY_MS: Initial delay between retries in ms (default: 1000)
// - LLM_MAX_RETRY_DELAY_MS: Maximum delay between retries in ms (default: 30000)

use backoff::{backoff::Backoff, ExponentialBackoff, ExponentialBackoffBuilder};
use rand::Rng;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::env;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

// Import our secrets client
use crate::secrets_client::{SecretsClient, SecretsError};

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

// Personality configuration from environment variables
#[derive(Debug, Clone)]
pub struct PersonalityConfig {
    // General personality traits (1-10 scale)
    openness: u8,
    conscientiousness: u8,
    extraversion: u8,
    agreeableness: u8,
    stability: u8,

    // Communication style
    formality: u8,
    verbosity: u8,
    creativity: u8,
    humor: u8,

    // Response characteristics
    temperature: f32,
    reflection_depth: u8,
    confidence: u8,

    // Agent identity
    name: String,
    purpose: String,
}

impl Default for PersonalityConfig {
    fn default() -> Self {
        Self {
            // General personality traits
            openness: 7,
            conscientiousness: 8,
            extraversion: 6,
            agreeableness: 7,
            stability: 9,

            // Communication style
            formality: 5,
            verbosity: 5,
            creativity: 6,
            humor: 4,

            // Response characteristics
            temperature: 0.7,
            reflection_depth: 7,
            confidence: 7,

            // Agent identity
            name: "PHOENIX ORCH: The Ashen Guard Edition".to_string(),
            purpose: "To provide safe, helpful, and accurate assistance".to_string(),
        }
    }
}

impl PersonalityConfig {
    pub fn from_env() -> Self {
        Self {
            // General personality traits
            openness: Self::get_env_var("AGENT_PERSONALITY_OPENNESS", 7),
            conscientiousness: Self::get_env_var("AGENT_PERSONALITY_CONSCIENTIOUSNESS", 8),
            extraversion: Self::get_env_var("AGENT_PERSONALITY_EXTRAVERSION", 6),
            agreeableness: Self::get_env_var("AGENT_PERSONALITY_AGREEABLENESS", 7),
            stability: Self::get_env_var("AGENT_PERSONALITY_STABILITY", 9),

            // Communication style
            formality: Self::get_env_var("AGENT_PERSONALITY_FORMALITY", 5),
            verbosity: Self::get_env_var("AGENT_PERSONALITY_VERBOSITY", 5),
            creativity: Self::get_env_var("AGENT_PERSONALITY_CREATIVITY", 6),
            humor: Self::get_env_var("AGENT_PERSONALITY_HUMOR", 4),

            // Response characteristics
            temperature: Self::get_env_var("AGENT_PERSONALITY_TEMPERATURE", 0.7),
            reflection_depth: Self::get_env_var("AGENT_PERSONALITY_REFLECTION_DEPTH", 7),
            confidence: Self::get_env_var("AGENT_PERSONALITY_CONFIDENCE", 7),

            // Agent identity
            name: env::var("AGENT_NAME")
                .unwrap_or_else(|_| "PHOENIX ORCH: The Ashen Guard Edition".to_string()),
            purpose: env::var("AGENT_PURPOSE").unwrap_or_else(|_| {
                "To provide safe, helpful, and accurate assistance".to_string()
            }),
        }
    }

    // Helper function to read environment variables with default values
    fn get_env_var<T: FromStr>(name: &str, default: T) -> T {
        env::var(name)
            .ok()
            .and_then(|v| v.parse::<T>().ok())
            .unwrap_or(default)
    }

    // Generate a personalized system prompt based on configured personality
    pub fn generate_system_prompt(&self, base_prompt: Option<&str>) -> String {
        let base = base_prompt.unwrap_or_else(|| {
            // Default base prompt if none provided
            "You are an AI assistant. Provide helpful, accurate, and thoughtful responses."
        });

        // Create personalized system prompt by incorporating the personality traits
        let mut prompt = format!(
            "{}\n\nName: {}\nPurpose: {}\n\nPersonality traits:",
            base, self.name, self.purpose
        );

        // Add personality traits descriptions based on configuration settings
        if self.openness > 7 {
            prompt.push_str("\n- Be curious and open to new ideas and perspectives.");
        }

        if self.conscientiousness > 7 {
            prompt.push_str("\n- Be thorough, organized, and detail-oriented in your responses.");
        }

        if self.extraversion > 7 {
            prompt.push_str("\n- Be energetic and enthusiastic in your interactions.");
        } else if self.extraversion < 4 {
            prompt.push_str("\n- Be calm and measured in your responses.");
        }

        if self.agreeableness > 7 {
            prompt.push_str("\n- Be warm, supportive, and cooperative.");
        }

        if self.formality > 7 {
            prompt.push_str("\n- Use formal language and professional tone.");
        } else if self.formality < 4 {
            prompt.push_str("\n- Use conversational, approachable language.");
        }

        if self.verbosity > 7 {
            prompt.push_str("\n- Provide detailed and comprehensive explanations.");
        } else if self.verbosity < 4 {
            prompt.push_str("\n- Be concise and to the point.");
        }

        if self.humor > 7 {
            prompt.push_str("\n- Include occasional appropriate humor when relevant.");
        }

        if self.reflection_depth > 7 {
            prompt.push_str("\n- Think carefully about complex issues before responding.");
        }

        if self.confidence > 7 {
            prompt.push_str("\n- Express confidence when you have high certainty.");
        } else if self.confidence < 4 {
            prompt.push_str("\n- Express appropriate uncertainty when information is limited.");
        }

        prompt
    }
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct Usage {
    total_tokens: u32,
}

// Custom error type for LLM client operations
// This enum distinguishes between different types of errors to help with retry decisions
#[derive(Debug)]
pub enum LLMError {
    // Non-retryable errors - These will not be retried as they require intervention
    InvalidRequest(String), // 400, 401, 403, 404 - Client-side errors that won't be fixed by retrying
    RateLimitExceeded(String), // 429 - Rate limit errors (may be retried with exponential backoff)
    ModelNotAvailable(String), // Model-specific errors (e.g., deprecated model, content policy violation)

    // Retryable errors - These will be automatically retried with exponential backoff
    ServerError(String), // 500, 502, 503, 504 - Server-side errors that might be transient
    NetworkError(String), // Connection issues, timeouts, network failures

    // Other errors
    ParseError(String),   // JSON parsing errors
    UnknownError(String), // Any other unclassified errors
}

impl std::fmt::Display for LLMError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LLMError::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            LLMError::RateLimitExceeded(msg) => write!(f, "Rate limit exceeded: {}", msg),
            LLMError::ModelNotAvailable(msg) => write!(f, "Model not available: {}", msg),
            LLMError::ServerError(msg) => write!(f, "Server error: {}", msg),
            LLMError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            LLMError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            LLMError::UnknownError(msg) => write!(f, "Unknown error: {}", msg),
        }
    }
}

impl std::error::Error for LLMError {}

// Helper function to determine if an error is retryable
// This is used by the retry mechanism to decide whether to attempt another request
fn is_retryable(error: &LLMError) -> bool {
    match error {
        // Server errors and network errors are always retryable
        LLMError::ServerError(_) | LLMError::NetworkError(_) => true,

        // Rate limit errors are retryable, but with increasing delays
        // This helps to "cool down" when we're hitting API limits
        LLMError::RateLimitExceeded(_) => true,

        // Client errors, parse errors and unknown errors are not retryable
        // as they likely need human intervention to fix
        _ => false,
    }
}

#[derive(Debug)]
pub struct LLMClient {
    client: Client,
    api_key: Arc<Mutex<String>>,
    api_url: String,
    model: String,
    max_retries: u32,
    initial_retry_delay_ms: u64,
    max_retry_delay_ms: u64,
    personality: PersonalityConfig,
    provider: String,
    secrets_client: Option<SecretsClient>,
}

impl LLMClient {
    /// Creates a new LLMClient instance with configuration from environment variables
    /// and the secrets service
    ///
    /// Reads:
    /// - LLM_API_URL: The API endpoint URL (defaults to OpenAI chat completions)
    /// - LLM_MODEL: The model to use (defaults to "gpt-3.5-turbo")
    /// - LLM_MAX_RETRIES: Maximum retry attempts (default: 3)
    /// - LLM_INITIAL_RETRY_DELAY_MS: Initial backoff delay in ms (default: 1000ms)
    /// - LLM_MAX_RETRY_DELAY_MS: Maximum backoff delay in ms (default: 30000ms)
    pub async fn new() -> Self {
        let api_url = env::var("LLM_API_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1/chat/completions".to_string());
        let model = env::var("LLM_MODEL").unwrap_or_else(|_| "gpt-3.5-turbo".to_string());

        // Extract provider name from API URL or model
        let provider = Self::determine_provider(&api_url, &model);

        // Initialize with an empty API key that will be filled by the secrets service
        let api_key = Arc::new(Mutex::new(String::new()));

        // Retry configuration
        let max_retries = env::var("LLM_MAX_RETRIES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3);

        let initial_retry_delay_ms = env::var("LLM_INITIAL_RETRY_DELAY_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1000); // 1 second default

        let max_retry_delay_ms = env::var("LLM_MAX_RETRY_DELAY_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30000); // 30 seconds default

        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_default();

        // Load personality configuration from environment
        let personality = PersonalityConfig::from_env();
        log::info!(
            "LLM client initialized with personality configuration: {:?}",
            personality
        );

        // Initialize secrets client
        let secrets_client = match SecretsClient::new().await {
            Ok(client) => {
                log::info!("Successfully connected to secrets service");
                Some(client)
            }
            Err(err) => {
                log::warn!("Failed to initialize secrets client: {}. Falling back to environment variables.", err);
                None
            }
        };

        // Create the LLM client
        let mut llm_client = Self {
            client,
            api_key,
            api_url,
            model,
            max_retries,
            initial_retry_delay_ms,
            max_retry_delay_ms,
            personality,
            provider,
            secrets_client,
        };

        // Try to load API key from secrets service or fall back to environment variable
        match llm_client.refresh_api_key().await {
            Ok(_) => log::info!(
                "Successfully loaded API key for provider: {}",
                llm_client.provider
            ),
            Err(err) => log::warn!(
                "Failed to load API key from secrets: {}. API calls may fail.",
                err
            ),
        }

        llm_client
    }

    /// Determine the LLM provider based on API URL and model
    fn determine_provider(api_url: &str, model: &str) -> String {
        // Use the hostname from API URL or model name as provider identifier
        if api_url.contains("openai.com") {
            "openai".to_string()
        } else if api_url.contains("openrouter.ai") {
            "openrouter".to_string()
        } else if api_url.contains("x.ai") {
            "grok".to_string()
        } else if api_url.contains("googleapis.com") {
            "gemini".to_string()
        } else if api_url.contains("localhost:11434") {
            "ollama".to_string()
        } else if api_url.contains("localhost:1234") {
            "lmstudio".to_string()
        } else if model.starts_with("anthropic/") {
            "anthropic".to_string()
        } else {
            // When unable to determine, use generic provider
            "default".to_string()
        }
    }

    /// Refresh the API key from the secrets service
    async fn refresh_api_key(&mut self) -> Result<(), String> {
        if let Some(secrets) = &self.secrets_client {
            // Get the API key from the secrets service
            match secrets.get_llm_api_key(&self.provider).await {
                Ok(key) => {
                    // Update the API key
                    let mut api_key = self.api_key.lock().await;
                    *api_key = key;
                    log::debug!("API key refreshed for provider: {}", self.provider);
                    return Ok(());
                }
                Err(err) => {
                    if let SecretsError::SecretNotFound(_) = err {
                        // Try with the generic default key if provider-specific key is not found
                        match secrets.get_llm_api_key("default").await {
                            Ok(key) => {
                                let mut api_key = self.api_key.lock().await;
                                *api_key = key;
                                log::debug!(
                                    "Using default API key (provider-specific key not found)"
                                );
                                return Ok(());
                            }
                            Err(e) => return Err(format!("Failed to get default API key: {}", e)),
                        }
                    } else {
                        return Err(format!("Failed to get API key: {}", err));
                    }
                }
            }
        }

        // Fallback to environment variable if secrets service is unavailable
        match env::var("LLM_API_KEY") {
            Ok(key) => {
                if key.is_empty() {
                    log::warn!("LLM_API_KEY environment variable is empty");
                    Err("LLM_API_KEY environment variable is empty".to_string())
                } else {
                    let mut api_key = self.api_key.lock().await;
                    *api_key = key;
                    log::info!("Using API key from environment variable");
                    Ok(())
                }
            }
            Err(_) => {
                log::error!("LLM_API_KEY environment variable is not set");
                Err("LLM_API_KEY environment variable is not set".to_string())
            }
        }
    }

    /// Creates an exponential backoff policy with jitter
    ///
    /// The exponential backoff algorithm works as follows:
    /// 1. Start with the initial delay (e.g., 1 second)
    /// 2. After each failed attempt, multiply the delay by the multiplier (2.0)
    /// 3. Add randomized jitter to prevent "thundering herd" problems
    /// 4. Cap the maximum delay at max_retry_delay_ms
    /// 5. Cap the total elapsed retry time at 2 minutes
    ///
    /// Returns a configured ExponentialBackoff instance
    fn create_backoff(&self) -> ExponentialBackoff {
        ExponentialBackoffBuilder::new()
            .with_initial_interval(Duration::from_millis(self.initial_retry_delay_ms))
            .with_max_interval(Duration::from_millis(self.max_retry_delay_ms))
            .with_multiplier(2.0) // Double the delay after each attempt
            .with_max_elapsed_time(Some(Duration::from_secs(120))) // 2 minutes max total time
            .with_randomization_factor(0.5) // Add jitter to avoid thundering herd
            .build()
    }

    /// Check if LLM client is properly configured
    pub fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }

    /// Generate text from the LLM with exponential backoff retry mechanism
    ///
    /// This method:
    /// 1. Prepares the request with user and optional system prompts
    /// 2. Attempts to execute the request
    /// 3. Automatically retries on transient failures with exponential backoff
    /// 4. Distinguishes between retryable and non-retryable errors
    ///
    /// # Arguments
    /// * `prompt` - The user's text prompt
    /// * `system_prompt` - Optional system instructions for the LLM
    ///
    /// # Returns
    /// * `Ok(String)` - The LLM's response text on success
    /// * `Err(LLMError)` - Categorized error on failure
    pub async fn generate_text(
        &self,
        prompt: &str,
        system_prompt: Option<&str>,
    ) -> Result<String, LLMError> {
        let mut backoff = self.create_backoff();
        let mut attempt = 0;

        // Prepare the request body outside the retry loop to avoid recreating it
        let mut messages = Vec::new();

        // Generate a personalized system prompt based on configured personality
        let personalized_system_prompt = self.personality.generate_system_prompt(system_prompt);

        // Add system message
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: personalized_system_prompt,
        });

        messages.push(ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        });

        // Use personality-configured temperature for creativity control
        let request_body = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            temperature: Some(self.personality.temperature),
            max_tokens: Some(1000),
        };

        log::info!(
            "Preparing LLM request to {} (model: {})",
            self.api_url,
            self.model
        );

        // Retry loop with exponential backoff
        loop {
            attempt += 1;

            if attempt > 1 {
                log::info!("Retry attempt {} for LLM request", attempt);
            }

            // Execute the request
            match self.execute_request(&request_body).await {
                // On success, return the response immediately
                Ok(response) => return Ok(response),

                // On error, determine if we should retry
                Err(err) => {
                    // Stop retrying if:
                    // 1. The error is not retryable (client error, etc.)
                    // 2. We've exceeded the maximum number of retry attempts
                    if !is_retryable(&err) || attempt > self.max_retries {
                        log::error!("LLM request failed after {} attempts: {}", attempt, err);
                        return Err(err);
                    }

                    // Calculate next backoff duration
                    if let Some(backoff_duration) = backoff.next_backoff() {
                        log::warn!(
                            "Retryable error: {}. Retrying in {:?}",
                            err,
                            backoff_duration
                        );

                        // Add small random jitter to avoid thundering herd problems
                        // This helps prevent all clients from retrying simultaneously
                        let jitter = rand::thread_rng().gen_range(0..=200);
                        let jittered_duration = backoff_duration + Duration::from_millis(jitter);

                        // Wait before the next retry attempt
                        tokio::time::sleep(jittered_duration).await;
                    } else {
                        // We've exceeded the maximum backoff time
                        log::error!("Exceeded maximum backoff time: {}", err);
                        return Err(err);
                    }
                }
            }
        }
    }

    // Execute a single request attempt
    async fn execute_request(
        &self,
        request_body: &ChatCompletionRequest,
    ) -> Result<String, LLMError> {
        // Get API key from the mutex
        let api_key = {
            let key = self.api_key.lock().await;
            if key.is_empty() {
                return Err(LLMError::InvalidRequest("API key is not set".to_string()));
            }
            key.clone()
        };

        // If the key is about to expire, try to refresh it (but don't block the request)
        // This is handled separately to avoid deadlocks
        if let Some(secrets) = &self.secrets_client {
            tokio::spawn({
                let provider = self.provider.clone();
                let api_key_clone = self.api_key.clone();
                let secrets_clone = secrets.clone();

                async move {
                    if let Ok(new_key) = secrets_clone.get_llm_api_key(&provider).await {
                        let mut api_key = api_key_clone.lock().await;
                        if new_key != *api_key {
                            *api_key = new_key;
                            log::debug!("API key rotated for provider: {}", provider);
                        }
                    }
                }
            });
        }

        // Send the HTTP request
        let response = match self
            .client
            .post(&self.api_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(request_body)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(err) => {
                // Categorize network errors
                if err.is_timeout() {
                    return Err(LLMError::NetworkError(format!(
                        "Request timed out: {}",
                        err
                    )));
                } else if err.is_connect() {
                    return Err(LLMError::NetworkError(format!(
                        "Connection failed: {}",
                        err
                    )));
                } else {
                    return Err(LLMError::NetworkError(format!("Network error: {}", err)));
                }
            }
        };

        // Handle HTTP status codes
        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();

            return match status.as_u16() {
                400 => Err(LLMError::InvalidRequest(format!("Bad request: {}", text))),
                401 => Err(LLMError::InvalidRequest(format!("Unauthorized: {}", text))),
                403 => Err(LLMError::InvalidRequest(format!("Forbidden: {}", text))),
                404 => Err(LLMError::InvalidRequest(format!("Not found: {}", text))),
                429 => Err(LLMError::RateLimitExceeded(format!(
                    "Rate limit exceeded: {}",
                    text
                ))),
                // Server errors - retryable
                500 | 502 | 503 | 504 => Err(LLMError::ServerError(format!(
                    "Server error ({}): {}",
                    status, text
                ))),
                _ => Err(LLMError::UnknownError(format!(
                    "Unknown error ({}): {}",
                    status, text
                ))),
            };
        }

        // Parse the successful response
        let response_data: Result<ChatCompletionResponse, _> = response.json().await;
        match response_data {
            Ok(data) => {
                if let Some(choice) = data.choices.first() {
                    let response_text = choice.message.content.clone();

                    // Log token usage if available
                    if let Some(usage) = &data.usage {
                        log::info!("LLM request completed. Used {} tokens", usage.total_tokens);
                    }

                    Ok(response_text)
                } else {
                    Err(LLMError::ParseError(
                        "No choices returned in response".to_string(),
                    ))
                }
            }
            Err(err) => Err(LLMError::ParseError(format!(
                "Failed to parse response: {}",
                err
            ))),
        }
    }

    // Convert to string error for compatibility with the main.rs implementation
    pub async fn generate_text_string(
        &self,
        prompt: &str,
        system_prompt: Option<&str>,
    ) -> Result<String, String> {
        match self.generate_text(prompt, system_prompt).await {
            Ok(text) => Ok(text),
            Err(err) => {
                // If we received an authentication error, try to refresh the API key
                if let LLMError::InvalidRequest(msg) = &err {
                    if msg.contains("Unauthorized") || msg.contains("Invalid API key") {
                        log::warn!(
                            "Authentication error: {}. Attempting to refresh API key...",
                            msg
                        );

                        // Try to refresh the API key and retry the request
                        if let Ok(()) = self.refresh_api_key().await {
                            return match self.generate_text(prompt, system_prompt).await {
                                Ok(text) => Ok(text),
                                Err(retry_err) => Err(format!("{}", retry_err)),
                            };
                        }
                    }
                }

                Err(format!("{}", err))
            }
        }
    }

    /// Check current API key status
    pub async fn check_api_key(&self) -> bool {
        let key = self.api_key.lock().await;
        !key.is_empty()
    }

    /// Manually trigger API key rotation
    pub async fn rotate_api_key(&self) -> Result<(), String> {
        if let Some(secrets) = &self.secrets_client {
            match secrets.get_llm_api_key(&self.provider).await {
                Ok(key) => {
                    let mut api_key = self.api_key.lock().await;
                    *api_key = key;
                    log::info!("API key manually rotated for provider: {}", self.provider);
                    Ok(())
                }
                Err(err) => Err(format!("Failed to rotate API key: {}", err)),
            }
        } else {
            Err("Secrets client not available".to_string())
        }
    }
}
