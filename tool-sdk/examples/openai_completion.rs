//! OpenAI Chat Completion Example
//!
//! This example demonstrates how to use the OpenAI client to send chat completion requests.
//!
//! To run this example:
//! ```
//! PHOENIX_OPENAI_API_KEY=your_api_key cargo run --example openai_completion
//! ```

use tool_sdk::{
    config::{ConfigProvider, EnvConfigProvider},
    error::Result,
    openai_client,
    services::openai::{ChatCompletionRequest, ChatMessage},
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    println!("OpenAI Chat Completion Example");

    // Create a config provider that reads from environment variables
    let config_provider = EnvConfigProvider::new()
        .with_prefix("PHOENIX")
        .with_namespace("OPENAI");

    // Load API key from environment
    let api_key = config_provider.get_string_or("API_KEY", "");
    if api_key.is_empty() {
        eprintln!("Please set PHOENIX_OPENAI_API_KEY environment variable");
        std::process::exit(1);
    }

    // Create an OpenAI client
    let client = openai_client().api_key(api_key).build()?;

    // Create a chat completion request
    let request = ChatCompletionRequest {
        model: "gpt-3.5-turbo".to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are a helpful assistant.".to_string(),
                name: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: "Hello, can you tell me a fun fact about Rust programming?".to_string(),
                name: None,
            },
        ],
        temperature: Some(0.7),
        max_tokens: Some(100),
        ..Default::default()
    };

    println!("Sending request to OpenAI...");

    // Send the request
    let response = client.chat_completion(request).await?;

    // Print the response
    println!("\nResponse from OpenAI:");
    if let Some(choice) = response.choices.first() {
        if let Some(content) = &choice.message.content {
            println!("{}", content);
        } else {
            println!("No content in response");
        }
    } else {
        println!("No choices in response");
    }

    // Print token usage
    println!("\nToken usage:");
    println!("  Prompt tokens: {}", response.usage.prompt_tokens);
    println!("  Completion tokens: {}", response.usage.completion_tokens);
    println!("  Total tokens: {}", response.usage.total_tokens);

    Ok(())
}
