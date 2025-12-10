//! SerpAPI Search Example
//!
//! This example demonstrates how to use the SerpAPI client to perform searches.
//! 
//! To run this example:
//! ```
//! PHOENIX_SERPAPI_API_KEY=your_api_key cargo run --example serpapi_search
//! ```

use tool_sdk::{
    serpapi_client,
    services::serpapi::GoogleSearchParams,
    config::{EnvConfigProvider, ConfigProvider},
    error::Result,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    
    println!("SerpAPI Search Example");
    
    // Create a config provider that reads from environment variables
    let config_provider = EnvConfigProvider::new()
        .with_prefix("PHOENIX")
        .with_namespace("SERPAPI");
    
    // Load API key from environment
    let api_key = config_provider.get_string_or("API_KEY", "");
    if api_key.is_empty() {
        eprintln!("Please set PHOENIX_SERPAPI_API_KEY environment variable");
        std::process::exit(1);
    }
    
    // Create a SerpAPI client
    let client = serpapi_client()
        .api_key(api_key)
        .build()?;
    
    // Create a search request
    let search_params = GoogleSearchParams {
        q: "Rust programming language".to_string(),
        num: Some(5),
        hl: Some("en".to_string()),
        gl: Some("us".to_string()),
        ..Default::default()
    };
    
    println!("Sending search request to SerpAPI...");
    
    // Send the request
    let response = client.google_search(search_params).await?;
    
    // Print the search metadata
    if let Some(metadata) = &response.search_metadata {
        println!("\nSearch Metadata:");
        if let Some(id) = &metadata.id {
            println!("  ID: {}", id);
        }
        if let Some(status) = &metadata.status {
            println!("  Status: {}", status);
        }
        if let Some(time) = &metadata.total_time_taken {
            println!("  Time taken: {:.2} seconds", time);
        }
    }
    
    // Print organic results
    if let Some(results) = &response.organic_results {
        println!("\nSearch Results:");
        
        for (i, result) in results.iter().enumerate() {
            println!("\nResult {}:", i + 1);
            
            if let Some(title) = &result.title {
                println!("  Title: {}", title);
            }
            
            if let Some(link) = &result.link {
                println!("  URL: {}", link);
            }
            
            if let Some(snippet) = &result.snippet {
                println!("  Snippet: {}", snippet);
            }
            
            println!("");
        }
        
        println!("Total results: {}", results.len());
    } else {
        println!("\nNo organic results found");
    }
    
    // Print related questions if any
    if let Some(questions) = &response.related_questions {
        println!("\nPeople also ask:");
        
        for (i, question) in questions.iter().enumerate() {
            if let Some(q) = &question.question {
                println!("  {}. {}", i + 1, q);
            }
        }
    }
    
    Ok(())
}