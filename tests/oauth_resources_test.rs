//! OAuth resource wire contracts pinned to official Python SDK 18f25547.
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};
use threatflux_anthropic_sdk::{
    oauth::resources::{
        FederationMatch, IssuerCreate, IssuerUpdate, Jwks, JwksInline, OAuthListParams,
        OrganizationRole, RuleCreate, RuleUpdate, ServiceAccountCreate, ServiceAccountUpdate,
    },
    OAuthAdminClient, OAuthConfig, OAuthPrincipal, OAuthToken, PaginationLimits, RequestOptions,
};
use wiremock::{
    matchers::{body_json, header, method, path, query_param},
    Mock, MockServer, ResponseTemplate,
};

#[path = "oauth_resources_test/lifecycle.rs"]
mod lifecycle;
#[path = "oauth_resources_test/memberships.rs"]
mod memberships;
#[path = "oauth_resources_test/support.rs"]
mod support;
#[path = "oauth_resources_test/validation.rs"]
mod validation;

use support::*;
