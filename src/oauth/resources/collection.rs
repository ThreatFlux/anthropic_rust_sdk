//! Shared collection transport and bounded OAuth pagination.

use super::{models::*, validation::*};
use crate::{
    error::{AnthropicError, Result},
    types::{HttpMethod, PageStream, PaginationLimits, RequestOptions},
};

use super::memberships::OAuthWorkspacesApi;
use crate::oauth::OAuthAdminClient;
use std::marker::PhantomData;

/// Common collection behavior for the three organization resources.
pub trait OAuthResource: serde::de::DeserializeOwned + Send + Sync + 'static {
    /// API collection path under `/organizations`.
    const COLLECTION: &'static str;
    /// Stable identifier used to detect pagination without item progress.
    fn resource_id(&self) -> &str;
}
impl OAuthResource for ServiceAccount {
    const COLLECTION: &'static str = "service_accounts";
    fn resource_id(&self) -> &str {
        &self.id
    }
}
impl OAuthResource for FederationIssuer {
    const COLLECTION: &'static str = "federation_issuers";
    fn resource_id(&self) -> &str {
        &self.id
    }
}
impl OAuthResource for FederationRule {
    const COLLECTION: &'static str = "federation_rules";
    fn resource_id(&self) -> &str {
        &self.id
    }
}

/// Typed OAuth collection client.
pub struct OAuthResources<T> {
    pub(super) client: OAuthAdminClient,
    marker: PhantomData<T>,
}
impl<T> Clone for OAuthResources<T> {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            marker: PhantomData,
        }
    }
}

/// Organization factories sharing one credential cache and transport.
#[derive(Clone)]
pub struct OAuthOrganizationApi {
    pub(super) client: OAuthAdminClient,
}
impl OAuthOrganizationApi {
    pub(crate) fn new(client: OAuthAdminClient) -> Self {
        Self { client }
    }
    /// Service-account lifecycle and workspace membership.
    pub fn service_accounts(&self) -> OAuthResources<ServiceAccount> {
        OAuthResources::new(self.client.clone())
    }
    /// Federation issuer lifecycle.
    pub fn federation_issuers(&self) -> OAuthResources<FederationIssuer> {
        OAuthResources::new(self.client.clone())
    }
    /// Federation rule lifecycle and workspace binding.
    pub fn federation_rules(&self) -> OAuthResources<FederationRule> {
        OAuthResources::new(self.client.clone())
    }
    /// Workspace-side service-account membership clients.
    pub fn workspaces(&self) -> OAuthWorkspacesApi {
        OAuthWorkspacesApi {
            client: self.client.clone(),
        }
    }
}

impl<T: OAuthResource> OAuthResources<T> {
    fn new(client: OAuthAdminClient) -> Self {
        Self {
            client,
            marker: PhantomData,
        }
    }
    pub(super) fn path(id: Option<&str>) -> Result<String> {
        let mut path = format!("/organizations/{}", T::COLLECTION);
        if let Some(id) = id {
            validate_id(id)?;
            path.push('/');
            path.push_str(id);
        }
        Ok(path)
    }
    /// Retrieve one resource.
    pub async fn get(&self, id: &str, options: Option<RequestOptions>) -> Result<T> {
        self.client
            .request(HttpMethod::Get, &Self::path(Some(id))?, None, options)
            .await
    }
    /// Soft-archive a resource. Archive referencing rules before accounts/issuers.
    pub async fn archive(&self, id: &str, options: Option<RequestOptions>) -> Result<T> {
        self.client
            .request(
                HttpMethod::Post,
                &format!("{}/archive", Self::path(Some(id))?),
                None,
                options,
            )
            .await
    }
    /// Retrieve a page, preserving endpoint filters.
    pub async fn list(
        &self,
        params: OAuthListParams,
        options: Option<RequestOptions>,
    ) -> Result<OAuthPage<T>> {
        if params.page.as_deref() == Some("") {
            return Err(AnthropicError::invalid_input(
                "OAuth page token must not be empty",
            ));
        }
        if params
            .limit
            .is_some_and(|limit| !(1..=100).contains(&limit))
        {
            return Err(AnthropicError::invalid_input(
                "OAuth page limit must be 1–100",
            ));
        }
        if params.issuer_id.is_some() && T::COLLECTION != "federation_rules" {
            return Err(AnthropicError::invalid_input(
                "Issuer filtering is only supported for federation rules",
            ));
        }
        let query = {
            let mut query = url::form_urlencoded::Serializer::new(String::new());
            if let Some(limit) = params.limit {
                query.append_pair("limit", &limit.to_string());
            }
            if let Some(page) = params.page {
                query.append_pair("page", &page);
            }
            if let Some(value) = params.include_archived {
                query.append_pair("include_archived", &value.to_string());
            }
            if let Some(issuer) = params.issuer_id {
                query.append_pair("issuer_id", &issuer);
            }
            query.finish()
        };
        let path = Self::path(None)?;
        let path = if query.is_empty() {
            path
        } else {
            format!("{path}?{query}")
        };
        self.client
            .request(HttpMethod::Get, &path, None, options)
            .await
    }
    /// Incremental bounded page traversal. Cursor cycles and limits return an error.
    pub fn pages(
        &self,
        params: OAuthListParams,
        limits: PaginationLimits,
        options: Option<RequestOptions>,
    ) -> Result<PageStream<T>> {
        let api = self.clone();
        let initial_cursor = params.page.clone();
        crate::api::utils::paginate(limits, initial_cursor, move |cursor| {
            let api = api.clone();
            let mut params = params.clone();
            let options = options.clone();
            params.page = cursor;
            async move {
                let page = api.list(params, options).await?;
                let item_ids = page
                    .data
                    .iter()
                    .map(|item| item.resource_id().to_owned())
                    .collect();
                Ok(crate::api::utils::TraversalPage {
                    data: page.data,
                    next_cursor: page.next_page,
                    item_ids,
                })
            }
        })
    }
    /// Collect bounded traversal with explicit limits.
    pub async fn list_all_with_limits(
        &self,
        params: OAuthListParams,
        limits: PaginationLimits,
        options: Option<RequestOptions>,
    ) -> Result<Vec<T>> {
        self.pages(params, limits, options)?.collect_items().await
    }

    /// Collect resources with the finite default traversal ceilings.
    pub async fn list_all(&self, options: Option<RequestOptions>) -> Result<Vec<T>> {
        self.list_all_with_limits(
            OAuthListParams::default(),
            PaginationLimits::default(),
            options,
        )
        .await
    }
}
