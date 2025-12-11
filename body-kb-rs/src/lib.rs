use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod doc_store;
pub mod validation;
pub mod vector_search;

/// Core Body Knowledge Base service implementation
pub struct BodyKnowledgeBase {
    // Core state
    state: Arc<RwLock<KBState>>,
}

/// Internal KB state
#[derive(Debug, Default)]
struct KBState {
    // Sensor data and actuator states
    sensor_readings: HashMap<String, Vec<u8>>,
    actuator_states: HashMap<String, Vec<u8>>,

    // Metadata and indices
    metadata: HashMap<String, HashMap<String, String>>,
}

impl BodyKnowledgeBase {
    /// Create a new Body KB instance
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(KBState::default())),
        }
    }

    /// Initialize the knowledge base
    pub async fn initialize(&self) -> Result<()> {
        // Initialize storage and indices
        let mut state = self.state.write().await;
        state.sensor_readings = HashMap::new();
        state.actuator_states = HashMap::new();
        state.metadata = HashMap::new();

        Ok(())
    }

    /// Query the knowledge base
    pub async fn query(&self, query: &str, limit: u64) -> Result<QueryResult> {
        // Validate query parameters
        validation::validate_query(query, limit)?;

        // Get current state
        let state = self.state.read().await;

        // Collect matching results
        let mut results = Vec::new();
        let mut metadata = HashMap::new();

        // Add sensor readings matching query
        for (key, value) in &state.sensor_readings {
            if key.contains(query) {
                results.push(value.clone());
                if let Some(meta) = state.metadata.get(key) {
                    metadata.insert(key.clone(), meta.clone());
                }
            }
        }

        // Add actuator states matching query
        for (key, value) in &state.actuator_states {
            if key.contains(query) {
                results.push(value.clone());
                if let Some(meta) = state.metadata.get(key) {
                    metadata.insert(key.clone(), meta.clone());
                }
            }
        }

        // Apply limit
        results.truncate(limit as usize);

        Ok(QueryResult {
            results,
            count: results.len(),
            metadata,
        })
    }

    /// Store data in the knowledge base
    pub async fn store(
        &self,
        key: &str,
        value: Vec<u8>,
        metadata: HashMap<String, String>,
    ) -> Result<String> {
        // Validate inputs
        let (sanitized_key, sanitized_value, sanitized_metadata) =
            validation::validate_store_request(key, &value, &metadata)?;

        // Update state
        let mut state = self.state.write().await;

        // Determine if this is sensor or actuator data based on key prefix
        if sanitized_key.starts_with("sensor/") {
            state
                .sensor_readings
                .insert(sanitized_key.clone(), sanitized_value);
        } else if sanitized_key.starts_with("actuator/") {
            state
                .actuator_states
                .insert(sanitized_key.clone(), sanitized_value);
        } else {
            // Default to sensor data
            state
                .sensor_readings
                .insert(sanitized_key.clone(), sanitized_value);
        }

        // Store metadata
        state
            .metadata
            .insert(sanitized_key.clone(), sanitized_metadata);

        Ok(sanitized_key)
    }

    /// Retrieve data from the knowledge base
    pub async fn retrieve(
        &self,
        key: &str,
        filters: &HashMap<String, String>,
    ) -> Result<RetrieveResult> {
        // Validate inputs
        let (sanitized_key, sanitized_filters) =
            validation::validate_retrieve_request(key, filters)?;

        // Get state
        let state = self.state.read().await;

        // Try to get value from both sensor and actuator data
        let value = state
            .sensor_readings
            .get(&sanitized_key)
            .or_else(|| state.actuator_states.get(&sanitized_key))
            .cloned();

        // Get metadata
        let metadata = state
            .metadata
            .get(&sanitized_key)
            .cloned()
            .unwrap_or_default();

        Ok(RetrieveResult {
            value,
            metadata,
            found: value.is_some(),
        })
    }

    /// Get service health status
    pub async fn health_check(&self) -> Result<bool> {
        // For now just check if we can access state
        let _state = self.state.read().await;
        Ok(true)
    }
}

/// Query result structure
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub results: Vec<Vec<u8>>,
    pub count: usize,
    pub metadata: HashMap<String, HashMap<String, String>>,
}

/// Retrieve result structure
#[derive(Debug, Clone)]
pub struct RetrieveResult {
    pub value: Option<Vec<u8>>,
    pub metadata: HashMap<String, String>,
    pub found: bool,
}

impl Default for BodyKnowledgeBase {
    fn default() -> Self {
        Self::new()
    }
}
