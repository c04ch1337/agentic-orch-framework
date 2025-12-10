use std::sync::Arc;
use axum::{
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
    extract::Path,
    http::header::AUTHORIZATION,
    body::{self, Body},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;

// Re-export functions from auth_client
pub use crate::auth_client::{
    init_auth_client,
    validate_token,
    check_permission,
    validate_api_key,
    revoke_token,
    generate_client_token,
    is_auth_healthy,
};

// Token data structure (local definition instead of phoenix_orch_proto)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenData {
    pub token: String,
    pub user_id: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub expires_at: i64,
}

// Permission maps for endpoints
static ENDPOINT_PERMISSIONS: Lazy<RwLock<HashMap<String, String>>> = Lazy::new(|| {
    let mut map = HashMap::new();
    
    // Default permissions for endpoints
    map.insert("/api/v1/execute".to_string(), "execute:invoke".to_string());
    map.insert("/api/v1/token".to_string(), "tokens:generate".to_string());
    
    // Protected admin endpoints
    map.insert("/api/v1/admin/users".to_string(), "admin:users:read".to_string());
    map.insert("/api/v1/admin/users/create".to_string(), "admin:users:create".to_string());
    map.insert("/api/v1/admin/users/update".to_string(), "admin:users:update".to_string());
    map.insert("/api/v1/admin/users/delete".to_string(), "admin:users:delete".to_string());
    
    RwLock::new(map)
});

// Error response structure
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthErrorResponse {
    pub error: String,
    pub code: u16,
}

/// Extract token from Authorization header
fn extract_token(req: &Request<Body>) -> Option<String> {
    let auth_header = req.headers()
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok());
    
    match auth_header {
        Some(auth) if auth.starts_with("Bearer ") => Some(auth[7..].to_string()),
        Some(auth) => Some(auth.to_string()), // Fallback for simple tokens or API keys
        None => None,
    }
}

/// Main authentication middleware
pub async fn auth_middleware(req: Request<Body>, next: Next) -> Result<Response, impl IntoResponse> {
    // Skip auth for some endpoints
    let path = req.uri().path();
    if path == "/" || path == "/health" {
        return Ok(next.run(req).await);
    }
    
    // Get token from header
    let token = match extract_token(&req) {
        Some(token) => token,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(AuthErrorResponse {
                    error: "Missing authentication token".to_string(),
                    code: 401,
                }),
            ));
        }
    };
    
    // Validate token
    match validate_token(&token).await {
        Ok(token_data) => {
            // Token is valid, create a new request with the validated token data
            // We need to clone the request to add token data to extensions
            let (parts, body) = req.into_parts();
            let mut req = Request::from_parts(parts, body);
            
            // Store token data in request extensions for later middleware
            req.extensions_mut().insert(token_data);
            
            // Continue to next middleware
            Ok(next.run(req).await)
        }
        Err(err) => {
            // First try to validate as API key for backward compatibility
            match validate_api_key(&token).await {
                Ok(token_data) => {
                    // API key is valid, create request with token data
                    let (parts, body) = req.into_parts();
                    let mut req = Request::from_parts(parts, body);
                    
                    // Store token data in request extensions
                    req.extensions_mut().insert(token_data);
                    
                    // Continue to next middleware
                    Ok(next.run(req).await)
                }
                Err(_) => {
                    // Both token and API key validation failed
                    Err((
                        StatusCode::UNAUTHORIZED,
                        Json(AuthErrorResponse {
                            error: format!("Invalid authentication token: {}", err),
                            code: 401,
                        }),
                    ))
                }
            }
        }
    }
}

/// Permission checking middleware - runs after auth_middleware
pub async fn permission_middleware(req: Request<Body>, next: Next) -> Result<Response, impl IntoResponse> {
    // Skip permission check for public endpoints
    let path = req.uri().path();
    if path == "/" || path == "/health" {
        return Ok(next.run(req).await);
    }
    
    // Get required permission for this endpoint
    let endpoint_perm = {
        let perms = ENDPOINT_PERMISSIONS.read().await;
        perms.get(path).cloned()
    };
    
    // If no permission required for this endpoint, continue
    if endpoint_perm.is_none() {
        return Ok(next.run(req).await);
    }
    
    // Get token from auth_middleware's extensions
    let token_data = match req.extensions().get::<TokenData>() {
        Some(data) => data,
        None => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: "Token data missing from request".to_string(),
                    code: 500,
                }),
            ));
        }
    };
    
    // Check permission
    let required_permission = endpoint_perm.unwrap();
    
    // If token has admin role, allow access to everything
    let has_admin = token_data.roles.contains(&"admin".to_string());
    if has_admin {
        return Ok(next.run(req).await);
    }
    
    // Otherwise check the specific permission
    match check_permission(&token_data.token, &required_permission).await {
        Ok(true) => {
            // User has permission, proceed
            Ok(next.run(req).await)
        }
        Ok(false) => {
            // User doesn't have required permission
            Err((
                StatusCode::FORBIDDEN,
                Json(AuthErrorResponse {
                    error: format!("Permission denied: {} required", required_permission),
                    code: 403,
                }),
            ))
        }
        Err(err) => {
            // Error occurred while checking permission
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AuthErrorResponse {
                    error: format!("Failed to check permissions: {}", err),
                    code: 500,
                }),
            ))
        }
    }
}

/// Initialize permission map with custom permissions
/// This can be used to update permissions at runtime
pub async fn init_permission_map(permissions: HashMap<String, String>) {
    let mut map = ENDPOINT_PERMISSIONS.write().await;
    *map = permissions;
}

/// Add a permission requirement for an endpoint
pub async fn add_endpoint_permission(endpoint: &str, permission: &str) {
    let mut map = ENDPOINT_PERMISSIONS.write().await;
    map.insert(endpoint.to_string(), permission.to_string());
}

/// Remove a permission requirement for an endpoint
pub async fn remove_endpoint_permission(endpoint: &str) {
    let mut map = ENDPOINT_PERMISSIONS.write().await;
    map.remove(endpoint);
}

/// Get the current permission map
pub async fn get_permission_map() -> HashMap<String, String> {
    let map = ENDPOINT_PERMISSIONS.read().await;
    map.clone()
}