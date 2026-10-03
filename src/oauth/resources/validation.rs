//! Local resource, scope, provider, and mutation validation.

use super::models::*;
use crate::error::{AnthropicError, Result};
use serde_json::Value;
use std::collections::HashMap;

pub(super) fn validate_id(id: &str) -> Result<()> {
    if id.is_empty()
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_-".contains(&byte))
    {
        return Err(AnthropicError::invalid_input("Invalid resource identifier"));
    }
    Ok(())
}
pub(super) fn validate_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 255
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"-".contains(&byte))
    {
        return Err(AnthropicError::invalid_input(
            "Resource names require 1–255 lowercase letters, digits or hyphens",
        ));
    }
    Ok(())
}
pub(super) fn validate_scope(scope: &str) -> Result<()> {
    if !matches!(scope, "workspace:developer" | "workspace:inference") {
        return Err(AnthropicError::invalid_input("OAuth API callers can only modify workspace developer/inference rules; use the Console for other scopes"));
    }
    Ok(())
}
pub(super) fn validate_lifetime(lifetime: Option<u32>) -> Result<()> {
    if lifetime.is_some_and(|value| !(60..=86400).contains(&value)) {
        return Err(AnthropicError::invalid_input(
            "Token lifetime must be 60–86400 seconds",
        ));
    }
    Ok(())
}
pub(super) fn validate_provider_url(value: &str) -> Result<()> {
    let url = url::Url::parse(value)
        .map_err(|_| AnthropicError::invalid_input("Invalid provider URL"))?;
    if url.scheme() != "https"
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || !matches!(url.host(),Some(url::Host::Domain(domain)) if domain.contains('.') && domain != "localhost")
    {
        return Err(AnthropicError::invalid_input(
            "Provider URLs require HTTPS public DNS names on port 443",
        ));
    }
    Ok(())
}
pub(super) fn validate_jwks(jwks: &Jwks) -> Result<()> {
    match jwks {
        Jwks::Discovery(value) => {
            validate_kind_and_extra(
                &value.kind,
                "discovery",
                &value.extra,
                &["type", "discovery_base", "ca_cert_pem"],
            )?;
            if let Some(url) = &value.discovery_base {
                validate_provider_url(url)?;
            }
        }
        Jwks::ExplicitUrl(value) => {
            validate_kind_and_extra(
                &value.kind,
                "explicit_url",
                &value.extra,
                &["type", "url", "ca_cert_pem"],
            )?;
            validate_provider_url(&value.url)?;
        }
        Jwks::Inline(value) => {
            validate_kind_and_extra(&value.kind, "inline", &value.extra, &["type", "keys"])?;
        }
        Jwks::Unknown(_) => {
            return Err(AnthropicError::invalid_input(
                "Unknown JWKS configuration cannot be sent by typed mutation methods",
            ))
        }
    }
    Ok(())
}
pub(super) fn workspace_query(params: OAuthListParams) -> Result<String> {
    if params.include_archived.is_some()
        || params.issuer_id.is_some()
        || params
            .limit
            .is_some_and(|limit| !(1..=100).contains(&limit))
        || params.page.as_deref() == Some("")
    {
        return Err(AnthropicError::invalid_input(
            "Unsupported workspace list options",
        ));
    }
    let mut query = url::form_urlencoded::Serializer::new(String::new());
    if let Some(page) = params.page {
        query.append_pair("page", &page);
    }
    if let Some(limit) = params.limit {
        query.append_pair("limit", &limit.to_string());
    }
    let query = query.finish();
    Ok(if query.is_empty() {
        query
    } else {
        format!("?{query}")
    })
}

pub(super) fn validate_workspace_role(role: &str) -> Result<()> {
    if !matches!(
        role,
        "workspace_admin"
            | "workspace_developer"
            | "workspace_restricted_developer"
            | "workspace_user"
    ) {
        return Err(AnthropicError::invalid_input(
            "Unsupported non-billing workspace role",
        ));
    }
    Ok(())
}

pub(super) fn validate_kind_and_extra(
    kind: &str,
    expected: &str,
    extra: &HashMap<String, Value>,
    reserved: &[&str],
) -> Result<()> {
    if kind != expected || reserved.iter().any(|key| extra.contains_key(*key)) {
        return Err(AnthropicError::invalid_input(
            "Typed federation payload has a conflicting discriminator or reserved extension key",
        ));
    }
    Ok(())
}

pub(super) fn validate_target(target: &ServiceAccountTarget) -> Result<()> {
    validate_kind_and_extra(
        &target.kind,
        "service_account",
        &target.extra,
        &["type", "service_account_id", "service_account_name"],
    )?;
    validate_id(&target.service_account_id)
}

pub(super) fn validate_match(matcher: &FederationMatch) -> Result<()> {
    if ["audience", "claims", "condition", "subject_prefix"]
        .iter()
        .any(|key| matcher.extra.contains_key(*key))
    {
        return Err(AnthropicError::invalid_input(
            "Federation matcher extension uses a reserved key",
        ));
    }
    let subject = matcher
        .subject_prefix
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty() && value.trim().chars().any(|ch| ch != '*'));
    let claims = matcher
        .claims
        .as_ref()
        .is_some_and(|claims| !claims.is_empty());
    let condition = matcher
        .condition
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty() && value.trim() != "true");
    if !subject && !claims && !condition {
        return Err(AnthropicError::invalid_input("Federation matching requires restrictive subject_prefix, claims, or condition; audience alone is insufficient"));
    }
    Ok(())
}

pub(super) fn validate_jwt_lifetime(lifetime: Option<u64>) -> Result<()> {
    if lifetime.is_some_and(|value| !(1..=176_400).contains(&value)) {
        return Err(AnthropicError::invalid_input(
            "Maximum JWT lifetime must be between 1 and 176400 seconds",
        ));
    }
    Ok(())
}

pub(super) fn validate_issuer_url(issuer: &str, jwks: &Jwks) -> Result<()> {
    if issuer.is_empty() {
        return Err(AnthropicError::invalid_input(
            "Issuer claim value must not be empty",
        ));
    }
    if let Jwks::Discovery(discovery) = jwks {
        if discovery.discovery_base.is_none() {
            validate_provider_url(issuer)?;
        }
    }
    Ok(())
}
