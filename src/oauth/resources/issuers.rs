//! Federation issuer lifecycle.

use super::{models::*, validation::*};
use crate::{
    error::{AnthropicError, Result},
    types::{HttpMethod, RequestOptions},
};

use super::collection::OAuthResources;

impl OAuthResources<FederationIssuer> {
    /// Register an issuer. Only discovery/JWKS fetch URLs require public HTTPS on port 443.
    pub async fn create(
        &self,
        request: IssuerCreate,
        options: Option<RequestOptions>,
    ) -> Result<FederationIssuer> {
        validate_name(&request.name)?;
        validate_issuer_url(&request.issuer_url, &request.jwks)?;
        validate_jwks(&request.jwks)?;
        validate_jwt_lifetime(request.max_jwt_lifetime_seconds)?;
        self.client
            .request(
                HttpMethod::Post,
                &Self::path(None)?,
                Some(serde_json::to_value(request)?),
                options,
            )
            .await
    }
    /// Update an issuer. Broader-scope rule references are checked by the server.
    pub async fn update(
        &self,
        id: &str,
        request: IssuerUpdate,
        options: Option<RequestOptions>,
    ) -> Result<FederationIssuer> {
        if let Some(name) = &request.name {
            validate_name(name)?;
        }
        if let Some(url) = &request.issuer_url {
            if let Some(jwks) = &request.jwks {
                validate_issuer_url(url, jwks)?;
            } else if url.is_empty() {
                return Err(AnthropicError::invalid_input(
                    "Issuer claim value must not be empty",
                ));
            }
            // The server knows the existing JWKS mode when an update omits it.
        }
        validate_jwt_lifetime(request.max_jwt_lifetime_seconds)?;
        if request.jwks_polling_disabled == Some(true) {
            return Err(AnthropicError::invalid_input(
                "jwks_polling_disabled accepts only false to resume polling",
            ));
        }
        if let Some(jwks) = &request.jwks {
            validate_jwks(jwks)?;
        }
        self.client
            .request(
                HttpMethod::Post,
                &Self::path(Some(id))?,
                Some(serde_json::to_value(request)?),
                options,
            )
            .await
    }
}
