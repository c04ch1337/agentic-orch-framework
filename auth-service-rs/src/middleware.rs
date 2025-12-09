// auth-service-rs/src/middleware.rs
//
// Authorization middleware for grpc and http services
// Provides:
// - Permission and role-based authorization middleware
// - Request attribution and identity tracking 
// - Capability-based security for service operations
// - JWT token validation and verification

use std::collections::HashMap;
use std::task::{Context, Poll};
use std::pin::Pin;
use std::sync::Arc;
use std::future::Future;

use tonic::{Request, Status};
use tonic::service::Interceptor;
use tonic::transport::Channel;
use http::{Response, HeaderMap, HeaderValue};
use tower::Service;
use tower::Layer;
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use tracing::{debug, error, info, warn, Span, trace};

use crate::jwt::Claims;
use crate::audit;
use crate::rbac::{RbacManager, PrincipalType};

// Permission requirement types
#[derive(Debug, Clone)]
pub enum Permission {
    // Requires a specific permission
    Required(String),
    
    // Requires any one of the permissions
    Any(Vec<String>),
    
    // Requires all of the permissions
    All(Vec<String>),
}

// Permission context for middleware use
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub principal_id: String,
    pub principal_type: String,
    pub token: Option<String>,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub request_id: Option<String>,
}

/// Authorization middleware configuration for gRPC services
pub struct AuthConfig {
    pub service_name: String,
    pub rbac_manager: Option<Arc<RbacManager>>,
}

// Token data for request context
#[derive(Debug, Clone)]
pub struct TokenData {
    pub token: String,
    pub claims: Claims,
}

// Middleware for validating JWT tokens in gRPC services
#[derive(Clone)]
pub struct JwtInterceptor {
    service_id: String,
}

impl JwtInterceptor {
    pub fn new(service_id: &str) -> Self {
        Self {
            service_id: service_id.to_string(),
        }
    }
}

impl Interceptor for JwtInterceptor {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        // Extract token from metadata
        let token = match request.metadata().get("authorization") {
            Some(t) => {
                let token_str = match t.to_str() {
                    Ok(v) => v,
                    Err(_) => return Err(Status::unauthenticated("Invalid authorization header")),
                };
                
                // Handle Bearer prefix
                if token_str.starts_with("Bearer ") {
                    token_str[7..].to_string()
                } else {
                    token_str.to_string()
                }
            }
            None => return Err(Status::unauthenticated("Missing authorization header")),
        };
        
        // Validate token (will happen asynchronously in actual request handler)
        // Here we just verify the token structure and add it to the request extensions
        // The actual validation and authorization will occur in the service
        
        // Add service ID to request extensions
        let service_id = self.service_id.clone();
        request.extensions_mut().insert(service_id);
        
        // Add token to request extensions
        request.extensions_mut().insert(token);
        
        // Add request ID if provided
        if let Some(req_id) = request.metadata().get("x-request-id") {
            if let Ok(req_id_str) = req_id.to_str() {
                request.extensions_mut().insert(req_id_str.to_string());
            }
        }
        
        Ok(request)
    }
}

// gRPC authorization middleware
pub struct AuthMiddleware<S> {
    inner: S,
    service_name: String,
    rbac_manager: Option<Arc<RbacManager>>,
}

impl<S> AuthMiddleware<S> {
    pub fn new(inner: S, config: AuthConfig) -> Self {
        Self {
            inner,
            service_name: config.service_name,
            rbac_manager: config.rbac_manager,
        }
    }
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for AuthMiddleware<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
    ResBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut request: Request<ReqBody>) -> Self::Future {
        // Take a clone of self.inner to move into the future
        let inner = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, inner);
        
        // Extract the token from extensions
        let token = request.extensions().get::<String>().cloned();
        
        // Get request method and path
        let method = request.method().clone();
        let uri = request.uri().clone();
        
        // Get service name
        let service_name = self.service_name.clone();
        
        // Get RBAC manager (if available)
        let rbac_manager = self.rbac_manager.clone();
        
        Box::pin(async move {
            // If no token, reject the request immediately
            let token = match token {
                Some(t) => t,
                None => {
                    // Token missing (should have been caught by interceptor)
                    let status = Status::unauthenticated("Missing token");
                    
                    // Since this is a gRPC service, we need to convert Status to the service's error type
                    // For this example, we'll just panic - in production you'd handle this conversion
                    panic!("Missing token: {}", status);
                }
            };
            
            // Validate token with auth service
            match validate_token(&token).await {
                Ok(claims) => {
                    // Token is valid, check permissions based on method and path
                    let principal_id = claims.sub.clone();
                    let resource = format!("{}/{}", service_name, uri.path());
                    let action = method.as_str().to_lowercase();
                    
                    // Check permissions if RBAC manager is available
                    let permitted = if let Some(rbac) = rbac_manager {
                        let principal_type = if claims.custom_claims.get("service_name").is_some() {
                            PrincipalType::Service
                        } else {
                            PrincipalType::User
                        };
                        
                        match rbac.check_permission(
                            &principal_id,
                            &principal_type,
                            &resource,
                            &action,
                            None,
                        ).await {
                            Ok(decision) => {
                                // Log the decision
                                audit::log_access_decision(
                                    &principal_id,
                                    &format!("{:?}", principal_type),
                                    &resource,
                                    &action,
                                    decision.allowed,
                                    &decision.reason,
                                    None,
                                ).await.ok();
                                
                                decision.allowed
                            },
                            Err(err) => {
                                // Log error and deny by default
                                error!("Permission check failed: {}", err);
                                
                                audit::log_access_decision(
                                    &principal_id,
                                    if claims.custom_claims.get("service_name").is_some() { "service" } else { "user" },
                                    &resource,
                                    &action,
                                    false,
                                    &format!("Permission check error: {}", err),
                                    None,
                                ).await.ok();
                                
                                false
                            }
                        }
                    } else {
                        // No RBAC manager, check scopes in token
                        let scopes = claims.scopes.as_ref().unwrap_or(&Vec::new());
                        let has_permission = scopes.contains(&format!("{}:{}", resource, action)) || 
                                            scopes.contains(&format!("{}:*", resource)) || 
                                            scopes.contains(&"*:*".to_string());
                        
                        if has_permission {
                            // Log the access
                            audit::log_access_decision(
                                &principal_id,
                                if claims.custom_claims.get("service_name").is_some() { "service" } else { "user" },
                                &resource,
                                &action,
                                true,
                                "Permission granted based on token scopes",
                                None,
                            ).await.ok();
                        } else {
                            // Log the denial
                            audit::log_access_decision(
                                &principal_id,
                                if claims.custom_claims.get("service_name").is_some() { "service" } else { "user" },
                                &resource,
                                &action,
                                false,
                                "Permission denied based on token scopes",
                                None,
                            ).await.ok();
                        }
                        
                        has_permission
                    };
                    
                    // If not permitted, reject the request
                    if !permitted {
                        panic!("Permission denied for {} on {}", method, uri.path());
                    }
                    
                    // Add principal ID and claims to request extensions
                    request.extensions_mut().insert(claims);
                    
                    // Proceed with the request
                    inner.call(request).await
                }
                Err(err) => {
                    // Token validation failed
                    let status = Status::unauthenticated(&format!("Invalid token: {}", err));
                    panic!("Token validation failed: {}", err);
                }
            }
        })
    }
}

// Layer for adding the authorization middleware
pub struct AuthLayer {
    config: AuthConfig,
}

impl AuthLayer {
    pub fn new(config: AuthConfig) -> Self {
        Self { config }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthMiddleware<S>;

    fn layer(&self, service: S) -> Self::Service {
        AuthMiddleware::new(service, self.config.clone())
    }
}

/// HTTP authorization middleware (for use with hyper/axum/warp)
pub struct HttpAuthMiddleware<S> {
    inner: S,
    service_name: String,
    rbac_manager: Option<Arc<RbacManager>>,
}

impl<S, ReqBody, ResBody> Service<http::Request<ReqBody>> for HttpAuthMiddleware<S>
where
    S: Service<http::Request<ReqBody>, Response = http::Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
    ResBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut request: http::Request<ReqBody>) -> Self::Future {
        // Take a clone of the inner service
        let inner = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, inner);
        
        // Extract authorization header
        let token = match request.headers().get("authorization") {
            Some(header) => {
                match header.to_str() {
                    Ok(auth) => {
                        if auth.starts_with("Bearer ") {
                            Some(auth[7..].to_string())
                        } else {
                            Some(auth.to_string())
                        }
                    }
                    Err(_) => None,
                }
            }
            None => None,
        };
        
        // Get method and path
        let method = request.method().clone();
        let path = request.uri().path().to_string();
        
        // Get service name
        let service_name = self.service_name.clone();
        
        // Get RBAC manager
        let rbac_manager = self.rbac_manager.clone();
        
        Box::pin(async move {
            // Check if authentication is required for this path
            // Some endpoints might be public (e.g., health checks)
            if path == "/health" || path == "/metrics" || path.starts_with("/public") {
                // Public endpoint, no auth required
                return inner.call(request).await;
            }
            
            // For everything else, validate the token
            match token {
                Some(token) => {
                    match validate_token(&token).await {
                        Ok(claims) => {
                            // Token is valid, check permissions
                            let principal_id = claims.sub.clone();
                            let resource = format!("{}{}", service_name, path);
                            let action = method.as_str().to_lowercase();
                            
                            // Check permissions if RBAC manager is available
                            let permitted = if let Some(rbac) = rbac_manager {
                                let principal_type = if claims.custom_claims.get("service_name").is_some() {
                                    PrincipalType::Service
                                } else {
                                    PrincipalType::User
                                };
                                
                                match rbac.check_permission(
                                    &principal_id,
                                    &principal_type,
                                    &resource,
                                    &action,
                                    None,
                                ).await {
                                    Ok(decision) => {
                                        // Log the decision
                                        audit::log_access_decision(
                                            &principal_id,
                                            &format!("{:?}", principal_type),
                                            &resource,
                                            &action,
                                            decision.allowed,
                                            &decision.reason,
                                            None,
                                        ).await.ok();
                                        
                                        decision.allowed
                                    },
                                    Err(err) => {
                                        // Log error and deny by default
                                        error!("Permission check failed: {}", err);
                                        
                                        audit::log_access_decision(
                                            &principal_id,
                                            if claims.custom_claims.get("service_name").is_some() { "service" } else { "user" },
                                            &resource,
                                            &action,
                                            false,
                                            &format!("Permission check error: {}", err),
                                            None,
                                        ).await.ok();
                                        
                                        false
                                    }
                                }
                            } else {
                                // No RBAC manager, check scopes in token
                                let scopes = claims.scopes.as_ref().unwrap_or(&Vec::new());
                                let has_permission = scopes.contains(&format!("{}:{}", resource, action)) || 
                                                    scopes.contains(&format!("{}:*", resource)) || 
                                                    scopes.contains(&"*:*".to_string());
                                
                                if has_permission {
                                    // Log the access
                                    audit::log_access_decision(
                                        &principal_id,
                                        if claims.custom_claims.get("service_name").is_some() { "service" } else { "user" },
                                        &resource,
                                        &action,
                                        true,
                                        "Permission granted based on token scopes",
                                        None,
                                    ).await.ok();
                                } else {
                                    // Log the denial
                                    audit::log_access_decision(
                                        &principal_id,
                                        if claims.custom_claims.get("service_name").is_some() { "service" } else { "user" },
                                        &resource,
                                        &action,
                                        false,
                                        "Permission denied based on token scopes",
                                        None,
                                    ).await.ok();
                                }
                                
                                has_permission
                            };
                            
                            // If not permitted, return 403 Forbidden
                            if !permitted {
                                let mut response = http::Response::new(hyper::Body::from("Permission denied"));
                                *response.status_mut() = http::StatusCode::FORBIDDEN;
                                return Ok(response.map(|_| unimplemented!("Body conversion not implemented"))); // HTTP conversion
                            }
                            
                            // Add principal ID and claims to request extensions
                            request.extensions_mut().insert(claims);
                            
                            // Proceed with the request
                            inner.call(request).await
                        }
                        Err(err) => {
                            // Token validation failed
                            let mut response = http::Response::new(hyper::Body::from(format!("Invalid token: {}", err)));
                            *response.status_mut() = http::StatusCode::UNAUTHORIZED;
                            Ok(response.map(|_| unimplemented!("Body conversion not implemented"))) // HTTP conversion
                        }
                    }
                }
                None => {
                    // No token provided, return 401 Unauthorized
                    let mut response = http::Response::new(hyper::Body::from("Missing authorization token"));
                    *response.status_mut() = http::StatusCode::UNAUTHORIZED;
                    Ok(response.map(|_| unimplemented!("Body conversion not implemented"))) // HTTP conversion
                }
            }
        })
    }
}

// Validate token against the auth service
async fn validate_token(token: &str) -> Result<Claims> {
    // Use the auth client to validate the token
    use crate::auth_client::{check_permission, validate_token as client_validate_token};
    
    // Validate the token
    let claims = match client_validate_token(token).await {
        Ok(token_data) => {
            // Convert token data to Claims
            Claims {
                sub: token_data.subject,
                iss: token_data.issuer.unwrap_or_default(),
                aud: token_data.audience.unwrap_or_default(),
                exp: token_data.expires_at.unwrap_or(0) as u64,
                nbf: token_data.not_before.unwrap_or(0) as u64,
                iat: token_data.issued_at.unwrap_or(0) as u64,
                jti: token_data.id.unwrap_or_default(),
                typ: token_data.token_type.unwrap_or_default(),
                roles: token_data.roles,
                scopes: Some(token_data.permissions),
                service_name: if token_data.token_type.unwrap_or_default() == "service" {
                    Some(token_data.subject.clone())
                } else {
                    None
                },
                user_name: if token_data.token_type.unwrap_or_default() == "user" {
                    token_data.username
                } else {
                    None
                },
                custom_claims: HashMap::new(), // We would populate this from metadata
            }
        }
        Err(err) => {
            return Err(anyhow!("Token validation failed: {}", err));
        }
    };
    
    // Log validation
    debug!("Token validated for subject {}", claims.sub);
    
    Ok(claims)
}

// Common function to register authorization middleware with a gRPC service
pub fn register_auth_middleware<S>(
    service: S, 
    service_name: &str,
    rbac_manager: Option<Arc<RbacManager>>,
) -> AuthMiddleware<S> {
    let config = AuthConfig {
        service_name: service_name.to_string(),
        rbac_manager,
    };
    
    AuthMiddleware::new(service, config)
}

// Create a JWT interceptor for a gRPC service
pub fn create_jwt_interceptor(service_id: &str) -> JwtInterceptor {
    JwtInterceptor::new(service_id)
}

// Authorization client trait for dependency injection and testing
#[async_trait]
pub trait AuthorizationClient: Send + Sync {
    async fn validate_token(&self, token: &str) -> Result<Claims>;
    async fn check_permission(&self, token: &str, resource: &str, action: &str) -> Result<bool>;
}

// Default implementation using the auth service
pub struct DefaultAuthorizationClient;

#[async_trait]
impl AuthorizationClient for DefaultAuthorizationClient {
    async fn validate_token(&self, token: &str) -> Result<Claims> {
        validate_token(token).await
    }
    
    async fn check_permission(&self, token: &str, permission: &str) -> Result<bool> {
        use crate::auth_client::check_permission as client_check_permission;
        
        match client_check_permission(token, permission).await {
            Ok(has_permission) => Ok(has_permission),
            Err(err) => Err(anyhow!("Permission check failed: {}", err)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::task::{Context, Poll, Waker};
    use std::pin::Pin;
    use tokio::runtime::Runtime;
    
    // Mock service for testing
    struct MockService<'a, ResBody> {
        response: &'a ResBody,
    }
    
    impl<'a, ResBody> MockService<'a, ResBody> {
        fn new(response: &'a ResBody) -> Self {
            Self { response }
        }
    }
    
    impl<'a, ResBody, ReqBody> tower::Service<Request<ReqBody>> for MockService<'a, ResBody> 
    where
        ResBody: Clone,
    {
        type Response = Response<ResBody>;
        type Error = &'static str;
        type Future = MockFuture<'a, ResBody>;
        
        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        
        fn call(&mut self, _req: Request<ReqBody>) -> Self::Future {
            MockFuture::new(self.response.clone())
        }
    }
    
    // Mock future for testing
    struct MockFuture<'a, T> {
        response: T,
        _phantom: std::marker::PhantomData<&'a T>,
    }
    
    impl<'a, T> MockFuture<'a, T> {
        fn new(response: T) -> Self {
            Self { 
                response,
                _phantom: std::marker::PhantomData,
            }
        }
    }
    
    impl<'a, T> Future for MockFuture<'a, T> {
        type Output = Result<Response<T>, &'static str>;
        
        fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            let response = Response::new(self.get_mut().response.clone());
            Poll::Ready(Ok(response))
        }
    }
    
    // Mock auth client for testing
    struct MockAuthClient;
    
    #[async_trait]
    impl AuthorizationClient for MockAuthClient {
        async fn validate_token(&self, token: &str) -> Result<Claims> {
            if token == "valid_token" {
                Ok(Claims {
                    sub: "test_user".to_string(),
                    iss: "test_issuer".to_string(),
                    aud: "test_audience".to_string(),
                    exp: 9999999999,
                    nbf: 0,
                    iat: 0,
                    jti: "test_id".to_string(),
                    typ: "access".to_string(),
                    roles: vec!["user".to_string()],
                    scopes: Some(vec!["test_service/resource:read".to_string()]),
                    service_name: None,
                    user_name: Some("Test User".to_string()),
                    custom_claims: HashMap::new(),
                })
            } else {
                Err(anyhow!("Invalid token"))
            }
        }
        
        async fn check_permission(&self, token: &str, permission: &str) -> Result<bool> {
            if token == "valid_token" && permission == "test_service/resource:read" {
                Ok(true)
            } else {
                Ok(false)
            }
        }
    }
}