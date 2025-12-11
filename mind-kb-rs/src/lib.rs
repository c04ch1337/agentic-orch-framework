//! Mind Knowledge Base Service
//! Provides vector storage and retrieval capabilities using Qdrant

pub mod validation;
pub mod vector_store;

// Re-export main types
pub use validation::{validate_content, validate_embedding, validate_metadata};
pub use vector_store::VectorStore;

// Generated proto modules
pub mod proto {
    pub mod agi_core {
        pub mod v1 {
            tonic::include_proto!("agi_core.v1");
        }
    }
}

// Re-export common types
pub use proto::agi_core::v1::{
    KnowledgeFragment,
    KnowledgeQuery,
    KnowledgeResponse,
    StoreRequest,
    StoreResponse,
};