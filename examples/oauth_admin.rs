//! Explicit bearer administration. Requires only ANTHROPIC_AUTH_TOKEN.
//! Reads existing resources; does not create accounts, trust rules, or memberships.

use threatflux_anthropic_sdk::{
    oauth::resources::OAuthListParams, OAuthAdminClient, PaginationLimits,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = OAuthAdminClient::from_env()?;
    let organization = client.organization();
    let limits = PaginationLimits::new(10, 1_000)?;
    let accounts = organization
        .service_accounts()
        .list_all_with_limits(OAuthListParams::default(), limits, None)
        .await?;
    let issuers = organization
        .federation_issuers()
        .list_all_with_limits(OAuthListParams::default(), limits, None)
        .await?;
    let rules = organization
        .federation_rules()
        .list_all_with_limits(OAuthListParams::default(), limits, None)
        .await?;
    // Resource details can contain private organization and trust configuration.
    println!(
        "Retrieved {} service accounts, {} federation issuers, and {} federation rules",
        accounts.len(),
        issuers.len(),
        rules.len()
    );
    Ok(())
}
