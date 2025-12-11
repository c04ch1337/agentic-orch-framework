// secrets-service-rs/src/service.rs
// gRPC service implementation for secrets management

use crate::auth::AuthManager;
use crate::vault_client::{SecretMetadata, VaultClient, VaultError, VaultOperations};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tonic::{Request, Response, Status};

// Import the generated proto service definitions
use crate::secrets_service::secrets_service_server::SecretsService;
use crate::secrets_service::{
    ListSecretsRequest, ListSecretsResponse, OperationResponse,
    SecretMetadata as ProtoSecretMetadata, SecretRequest, SecretResponse, ServiceAuthRequest,
    ServiceAuthResponse, SetSecretRequest, TokenRequest, TokenResponse,
};

// Service implementation
pub struct SecretsServiceImpl {
    vault_client: VaultClient,
    auth_manager: AuthManager,
}

impl SecretsServiceImpl {
    pub async fn new(vault_client: VaultClient) -> Self {
        let auth_manager = AuthManager::new().await;
        Self {
            vault_client,
            auth_manager,
        }
    }
}

#[tonic::async_trait]
impl SecretsService for SecretsServiceImpl {
    // Get a secret value by key
    async fn get_secret(
        &self,
        request: Request<SecretRequest>,
    ) -> Result<Response<SecretResponse>, Status> {
        let req = request.into_inner();

        log::info!("Get secret request for key: {}", req.key);

        // Implement proper validation and check authorization
        if req.service_id.is_empty() || req.auth_token.is_empty() {
            return Err(Status::invalid_argument("Missing service_id or auth_token"));
        }

        // Check if the token is authorized to access this secret
        let is_authorized = self
            .auth_manager
            .is_authorized(&req.auth_token, &format!("secret/{}", req.key), "read")
            .await;

        if !is_authorized {
            return Err(Status::permission_denied(
                "Not authorized to access this secret",
            ));
        }

        // Get the secret from Vault
        match self
            .vault_client
            .get_secret(&req.key, &req.auth_token)
            .await
        {
            Ok(value) => {
                // Return the secret value
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::from_secs(0))
                    .as_secs();

                Ok(Response::new(SecretResponse {
                    success: true,
                    secret_value: value,
                    error: String::new(),
                    expires_at: 0, // No expiration for this example
                }))
            }
            Err(err) => match err {
                VaultError::SecretNotFound(_) => {
                    Err(Status::not_found(format!("Secret not found: {}", req.key)))
                }
                VaultError::PermissionDenied(msg) => Err(Status::permission_denied(msg)),
                VaultError::AuthenticationError(msg) => Err(Status::unauthenticated(msg)),
                _ => {
                    log::error!("Error getting secret: {:?}", err);
                    Err(Status::internal(format!("Error getting secret: {}", err)))
                }
            },
        }
    }

    // Set a secret value
    async fn set_secret(
        &self,
        request: Request<SetSecretRequest>,
    ) -> Result<Response<OperationResponse>, Status> {
        let req = request.into_inner();

        log::info!("Set secret request for key: {}", req.key);

        // Implement proper validation
        if req.key.is_empty() || req.secret_value.is_empty() {
            return Err(Status::invalid_argument(
                "Key and secret value cannot be empty",
            ));
        }

        if req.service_id.is_empty() || req.auth_token.is_empty() {
            return Err(Status::invalid_argument("Missing service_id or auth_token"));
        }

        // Check if the token is authorized to set this secret
        let is_authorized = self
            .auth_manager
            .is_authorized(&req.auth_token, &format!("secret/{}", req.key), "write")
            .await;

        if !is_authorized {
            return Err(Status::permission_denied(
                "Not authorized to set this secret",
            ));
        }

        // Set the secret in Vault
        match self
            .vault_client
            .set_secret(
                &req.key,
                &req.secret_value,
                req.ttl as u64,
                req.metadata,
                &req.auth_token,
            )
            .await
        {
            Ok(_) => {
                // Return success
                Ok(Response::new(OperationResponse {
                    success: true,
                    error: String::new(),
                    request_id: uuid::Uuid::new_v4().to_string(),
                }))
            }
            Err(err) => match err {
                VaultError::PermissionDenied(msg) => Err(Status::permission_denied(msg)),
                VaultError::AuthenticationError(msg) => Err(Status::unauthenticated(msg)),
                _ => {
                    log::error!("Error setting secret: {:?}", err);
                    Err(Status::internal(format!("Error setting secret: {}", err)))
                }
            },
        }
    }

    // Delete a secret by key
    async fn delete_secret(
        &self,
        request: Request<SecretRequest>,
    ) -> Result<Response<OperationResponse>, Status> {
        let req = request.into_inner();

        log::info!("Delete secret request for key: {}", req.key);

        // Implement proper validation
        if req.key.is_empty() {
            return Err(Status::invalid_argument("Key cannot be empty"));
        }

        if req.service_id.is_empty() || req.auth_token.is_empty() {
            return Err(Status::invalid_argument("Missing service_id or auth_token"));
        }

        // Check if the token is authorized to delete this secret
        let is_authorized = self
            .auth_manager
            .is_authorized(&req.auth_token, &format!("secret/{}", req.key), "delete")
            .await;

        if !is_authorized {
            return Err(Status::permission_denied(
                "Not authorized to delete this secret",
            ));
        }

        // Delete the secret from Vault
        match self
            .vault_client
            .delete_secret(&req.key, &req.auth_token)
            .await
        {
            Ok(_) => {
                // Return success
                Ok(Response::new(OperationResponse {
                    success: true,
                    error: String::new(),
                    request_id: uuid::Uuid::new_v4().to_string(),
                }))
            }
            Err(err) => match err {
                VaultError::SecretNotFound(_) => {
                    Err(Status::not_found(format!("Secret not found: {}", req.key)))
                }
                VaultError::PermissionDenied(msg) => Err(Status::permission_denied(msg)),
                VaultError::AuthenticationError(msg) => Err(Status::unauthenticated(msg)),
                _ => {
                    log::error!("Error deleting secret: {:?}", err);
                    Err(Status::internal(format!("Error deleting secret: {}", err)))
                }
            },
        }
    }

    // List available secrets (metadata only, not values)
    async fn list_secrets(
        &self,
        request: Request<ListSecretsRequest>,
    ) -> Result<Response<ListSecretsResponse>, Status> {
        let req = request.into_inner();

        log::info!("List secrets request for path: {}", req.path_prefix);

        if req.service_id.is_empty() || req.auth_token.is_empty() {
            return Err(Status::invalid_argument("Missing service_id or auth_token"));
        }

        // Check if the token is authorized to list secrets
        let is_authorized = self
            .auth_manager
            .is_authorized(&req.auth_token, "secret", "list")
            .await;

        if !is_authorized {
            return Err(Status::permission_denied("Not authorized to list secrets"));
        }

        // List secrets from Vault
        match self
            .vault_client
            .list_secrets(&req.path_prefix, &req.auth_token)
            .await
        {
            Ok(secrets) => {
                // Convert to proto format
                let mut proto_secrets = Vec::new();

                for secret in secrets {
                    proto_secrets.push(ProtoSecretMetadata {
                        key: secret.key,
                        created_at: secret.created_at as i64,
                        updated_at: secret.updated_at as i64,
                        expires_at: secret.expires_at as i64,
                        metadata: secret.metadata,
                    });
                }

                // Return the list
                Ok(Response::new(ListSecretsResponse {
                    success: true,
                    error: String::new(),
                    secrets: proto_secrets,
                }))
            }
            Err(err) => match err {
                VaultError::PermissionDenied(msg) => Err(Status::permission_denied(msg)),
                VaultError::AuthenticationError(msg) => Err(Status::unauthenticated(msg)),
                _ => {
                    log::error!("Error listing secrets: {:?}", err);
                    Err(Status::internal(format!("Error listing secrets: {}", err)))
                }
            },
        }
    }

    // Generate a short-lived service token
    async fn generate_token(
        &self,
        request: Request<TokenRequest>,
    ) -> Result<Response<TokenResponse>, Status> {
        let req = request.into_inner();

        log::info!("Generate token request for service: {}", req.service_id);

        if req.service_id.is_empty() || req.service_secret.is_empty() {
            return Err(Status::invalid_argument(
                "Missing service_id or service_secret",
            ));
        }

        // Authenticate service and generate token
        let token_data = self
            .auth_manager
            .authenticate_service(&req.service_id, &req.service_secret)
            .await;

        match token_data {
            Some(data) => {
                // Return the token
                Ok(Response::new(TokenResponse {
                    success: true,
                    token: data.token,
                    expires_at: data.expires_at.try_into().unwrap_or(0),
                    granted_roles: data.roles,
                    error: String::new(),
                }))
            }
            None => {
                log::warn!("Authentication failed for service: {}", req.service_id);
                Err(Status::unauthenticated("Authentication failed"))
            }
        }
    }

    // Authenticate a service using role-based access
    async fn authenticate_service(
        &self,
        request: Request<ServiceAuthRequest>,
    ) -> Result<Response<ServiceAuthResponse>, Status> {
        let req = request.into_inner();

        log::info!(
            "Authenticate service request for: {}, resource: {}, action: {}",
            req.service_id,
            req.target_resource,
            req.action
        );

        if req.service_id.is_empty() || req.auth_token.is_empty() {
            return Err(Status::invalid_argument("Missing service_id or auth_token"));
        }

        // Check if the token is valid (authenticated)
        let token_data = self.auth_manager.verify_token(&req.auth_token).await;
        let authenticated = token_data.is_some();

        // Check if the token is authorized for the requested action on the resource
        let authorized = if authenticated {
            self.auth_manager
                .is_authorized(&req.auth_token, &req.target_resource, &req.action)
                .await
        } else {
            false
        };

        // Get permissions if authenticated
        let permissions = if authenticated {
            let token_data = token_data.unwrap();
            token_data.roles
        } else {
            Vec::new()
        };

        // Return authentication and authorization result
        Ok(Response::new(ServiceAuthResponse {
            authenticated,
            authorized,
            error: if !authenticated {
                "Authentication failed".to_string()
            } else if !authorized {
                "Not authorized for this action".to_string()
            } else {
                String::new()
            },
            permissions,
        }))
    }
}
