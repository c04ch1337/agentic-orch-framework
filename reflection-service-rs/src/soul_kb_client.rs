// reflection-service-rs/src/soul_kb_client.rs
// Client for interacting with the Soul-KB service to store learned lessons

use anyhow::{Result, anyhow};
use log::{info, error, warn, debug};
use serde::{Serialize, Deserialize};
use std::env;
use tonic::transport::Channel;
use tonic::Request;

use crate::agi_core::{
    StoreValueRequest, StoreValueResponse, CoreValue,
    soul_kb_service_client::SoulKbServiceClient,
    StoreRequest, StoreResponse,
    ValuePriority
};

use crate::reflection_logic::{LessonLearned, Constraint};

/// Client for Soul-KB operations
pub struct SoulKBClient {
    client: Option<SoulKbServiceClient<Channel>>,
    mock_mode: bool,
}

impl SoulKBClient {
    /// Create a new Soul-KB client
    pub async fn new() -> Self {
        // Get Soul-KB address from env or use default
        let addr = env::var("SOUL_KB_ADDR")
            .unwrap_or_else(|_| "http://soul-kb-rs:50061".to_string());
        
        // Try to connect to the Soul-KB service
        match SoulKbServiceClient::connect(addr.clone()).await {
            Ok(client) => {
                info!("Connected to Soul-KB at {}", addr);
                Self {
                    client: Some(client),
                    mock_mode: false,
                }
            },
            Err(e) => {
                warn!("Failed to connect to Soul-KB at {}: {}. Using mock client.", addr, e);
                Self {
                    client: None,
                    mock_mode: true,
                }
            }
        }
    }
    
    /// Store a lesson as a Core Value in Soul-KB
    pub async fn store_lesson(&mut self, lesson: &LessonLearned) -> Result<String> {
        if self.mock_mode {
            info!("[MOCK] Storing lesson: {}", lesson.lesson);
            return Ok("mock-lesson-id".to_string());
        }
        
        let client = match self.client.as_mut() {
            Some(client) => client,
            None => return Err(anyhow!("Soul-KB client not available")),
        };
        
        // Convert priority from 1-5 scale to ValuePriority enum
        let priority = match lesson.priority {
            1 => ValuePriority::PriorityLow as i32,
            2 => ValuePriority::PriorityMedium as i32,
            3 => ValuePriority::PriorityHigh as i32,
            4 => ValuePriority::PriorityCritical as i32,
            5 => ValuePriority::PriorityImmutable as i32,
            _ => ValuePriority::PriorityMedium as i32, // Default
        };
        
        // Format the lesson as a constraint rule
        let constraint = format!("When performing actions like '{}', ensure: {}", 
                                lesson.context, lesson.lesson);
        
        // Create a CoreValue from the lesson
        let lesson_value = CoreValue {
            value_id: lesson.id.clone(),
            name: format!("learned_lesson_{}", &lesson.id[..8]),
            description: lesson.lesson.clone(),
            priority,
            constraint,
            is_active: true,
            metadata: {
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("source".to_string(), "reflection_service".to_string());
                metadata.insert("timestamp".to_string(), lesson.timestamp.to_string());
                metadata.insert("context".to_string(), lesson.context.clone());
                metadata
            },
        };
        
        let request = Request::new(StoreValueRequest {
            value: Some(lesson_value),
        });
        
        // Store the value and handle the response
        match client.store_value(request).await {
            Ok(response) => {
                let response = response.into_inner();
                if response.success {
                    info!("Successfully stored lesson in Soul-KB with ID: {}", response.value_id);
                    Ok(response.value_id)
                } else {
                    error!("Failed to store lesson in Soul-KB");
                    Err(anyhow!("Failed to store lesson in Soul-KB"))
                }
            },
            Err(e) => {
                error!("Error storing lesson in Soul-KB: {}", e);
                Err(anyhow!("Error storing lesson in Soul-KB: {}", e))
            }
        }
    }
    
    /// Alternative method to store a lesson as a generic fact
    pub async fn store_fact(&mut self, key: &str, value: &[u8]) -> Result<String> {
        if self.mock_mode {
            info!("[MOCK] Storing fact with key: {}", key);
            return Ok("mock-fact-id".to_string());
        }
        
        let client = match self.client.as_mut() {
            Some(client) => client,
            None => return Err(anyhow!("Soul-KB client not available")),
        };
        
        let request = Request::new(StoreRequest {
            key: key.to_string(),
            value: value.to_vec(),
            metadata: {
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("source".to_string(), "reflection_service".to_string());
                metadata.insert("type".to_string(), "lesson_learned".to_string());
                metadata
            },
        });
        
        // Store the fact and handle the response
        match client.store(request).await {
            Ok(response) => {
                let response = response.into_inner();
                if response.success {
                    info!("Successfully stored fact in Soul-KB with ID: {}", response.stored_id);
                    Ok(response.stored_id)
                } else {
                    error!("Failed to store fact in Soul-KB");
                    Err(anyhow!("Failed to store fact in Soul-KB"))
                }
            },
            Err(e) => {
                error!("Error storing fact in Soul-KB: {}", e);
                Err(anyhow!("Error storing fact in Soul-KB: {}", e))
            }
        }
    }
    
    /// Check if the client is running in mock mode
    pub fn is_mock(&self) -> bool {
        self.mock_mode
    }
    
    /// Store a constraint rule in Soul-KB
    pub async fn store_constraint(&mut self, constraint: &Constraint) -> Result<String> {
        if self.mock_mode {
            info!("[MOCK] Storing constraint: {}", constraint.constraint);
            return Ok("mock-constraint-id".to_string());
        }
        
        let client = match self.client.as_mut() {
            Some(client) => client,
            None => return Err(anyhow!("Soul-KB client not available")),
        };
        
        // Always use high priority for constraints
        let priority = ValuePriority::PriorityHigh as i32;
        
        // Create a CoreValue from the constraint
        let constraint_value = CoreValue {
            value_id: constraint.id.clone(),
            name: format!("negative_constraint_{}", &constraint.id[..8]),
            description: constraint.constraint.clone(),
            priority,
            constraint: constraint.constraint.clone(),
            is_active: true,
            metadata: {
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("source".to_string(), "reflection_service".to_string());
                metadata.insert("timestamp".to_string(), constraint.timestamp.to_string());
                metadata.insert("context".to_string(), constraint.context.clone());
                metadata.insert("immediate_use".to_string(), constraint.immediate_use.to_string());
                metadata
            },
        };
        
        let request = Request::new(StoreValueRequest {
            value: Some(constraint_value),
        });
        
        // Store the value and handle the response
        match client.store_value(request).await {
            Ok(response) => {
                let response = response.into_inner();
                if response.success {
                    info!("Successfully stored constraint in Soul-KB with ID: {}", response.value_id);
                    Ok(response.value_id)
                } else {
                    error!("Failed to store constraint in Soul-KB");
                    Err(anyhow!("Failed to store constraint in Soul-KB"))
                }
            },
            Err(e) => {
                error!("Error storing constraint in Soul-KB: {}", e);
                Err(anyhow!("Error storing constraint in Soul-KB: {}", e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    
    #[tokio::test]
    async fn test_store_lesson_mock() {
        let mut client = SoulKBClient::new().await;
        
        let lesson = LessonLearned {
            id: Uuid::new_v4().to_string(),
            lesson: "Always validate inputs before processing".to_string(),
            context: "data processing".to_string(),
            priority: 3,
            timestamp: chrono::Utc::now().timestamp(),
        };
        
        let result = client.store_lesson(&lesson).await;
        assert!(result.is_ok());
    }
}