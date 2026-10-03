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
    for account in organization
        .service_accounts()
        .list_all_with_limits(OAuthListParams::default(), limits, None)
        .await?
    {
        println!(
            "{}: {} ({})",
            account.id, account.name, account.organization_role
        );
    }
    for issuer in organization
        .federation_issuers()
        .list_all_with_limits(OAuthListParams::default(), limits, None)
        .await?
    {
        println!("{}: {}", issuer.id, issuer.issuer_url);
    }
    for rule in organization
        .federation_rules()
        .list_all_with_limits(OAuthListParams::default(), limits, None)
        .await?
    {
        println!("{}: {} ({})", rule.id, rule.name, rule.oauth_scope);
    }
    Ok(())
}
