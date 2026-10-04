//! Account- and workspace-addressed membership contracts.

use super::*;

#[tokio::test]
async fn account_and_rule_workspace_subresources_use_expected_methods_and_dtos() {
    let server = MockServer::start().await;
    mount_membership_subresources(&server).await;
    let organization = client(&server, OAuthPrincipal::User).organization();
    let accounts = organization.service_accounts();
    assert_eq!(
        accounts
            .workspaces("svac_worker", OAuthListParams::default(), None)
            .await
            .unwrap()
            .data
            .len(),
        1
    );
    accounts
        .add_workspace("svac_worker", "wrkspc_test", "workspace_developer", None)
        .await
        .unwrap();
    assert_eq!(
        accounts
            .remove_workspace("svac_worker", "wrkspc_test", None)
            .await
            .unwrap()
            .service_account_id,
        "svac_worker"
    );
    let rules = organization.federation_rules();
    assert_eq!(
        rules
            .workspaces("fdrl_deploy", None)
            .await
            .unwrap()
            .data
            .len(),
        1
    );
    rules
        .add_workspace("fdrl_deploy", "wrkspc_test", None)
        .await
        .unwrap();
    assert_eq!(
        rules
            .remove_workspace("fdrl_deploy", "wrkspc_test", None)
            .await
            .unwrap()
            .federation_rule_id,
        "fdrl_deploy"
    );
}

#[tokio::test]
async fn workspace_addressed_account_memberships_cover_get_list_add_update_remove() {
    let server = MockServer::start().await;
    mount_workspace_memberships(&server).await;
    let api = client(&server, OAuthPrincipal::User)
        .organization()
        .workspaces()
        .service_accounts("wrkspc_test")
        .unwrap();
    api.list(OAuthListParams::default(), None).await.unwrap();
    api.get("svac_worker", None).await.unwrap();
    api.add("svac_worker", "workspace_developer", None)
        .await
        .unwrap();
    api.update("svac_worker", "workspace_user", None)
        .await
        .unwrap();
    api.remove("svac_worker", None).await.unwrap();
}

async fn mount_membership_subresources(server: &MockServer) {
    let accounts = "/v1/organizations/service_accounts/svac_worker/workspaces";
    let rules = "/v1/organizations/federation_rules/fdrl_deploy/workspaces";
    mount(
        server,
        "GET",
        accounts,
        None,
        json!({"data":[membership()],"next_page":null}),
    )
    .await;
    mount(
        server,
        "POST",
        accounts,
        Some(json!({"workspace_id":"wrkspc_test","workspace_role":"workspace_developer"})),
        membership(),
    )
    .await;
    // Synthetic Anthropic resource discriminator; this fixture contains no Google credentials.
    // nosemgrep: generic.secrets.security.detected-google-gcm-service-account.detected-google-gcm-service-account
    mount(server, "DELETE", &format!("{accounts}/wrkspc_test"), None, json!({"type":"service_account_workspace_member_deleted","service_account_id":"svac_worker","workspace_id":"wrkspc_test"})).await;
    mount(server, "GET", rules, None, json!({"data":[binding()]})).await;
    mount(
        server,
        "POST",
        rules,
        Some(json!({"workspace_id":"wrkspc_test"})),
        binding(),
    )
    .await;
    mount(server, "DELETE", &format!("{rules}/wrkspc_test"), None, json!({"type":"federation_rule_workspace_deleted","federation_rule_id":"fdrl_deploy","workspace_id":"wrkspc_test"})).await;
}

async fn mount_workspace_memberships(server: &MockServer) {
    let base = "/v1/organizations/workspaces/wrkspc_test/service_accounts";
    mount(
        server,
        "GET",
        base,
        None,
        json!({"data":[membership()],"next_page":null}),
    )
    .await;
    mount(
        server,
        "GET",
        &format!("{base}/svac_worker"),
        None,
        membership(),
    )
    .await;
    mount(
        server,
        "POST",
        base,
        Some(json!({"service_account_id":"svac_worker","workspace_role":"workspace_developer"})),
        membership(),
    )
    .await;
    mount(
        server,
        "POST",
        &format!("{base}/svac_worker"),
        Some(json!({"workspace_role":"workspace_user"})),
        membership(),
    )
    .await;
    // Synthetic Anthropic resource discriminator; this fixture contains no Google credentials.
    // nosemgrep: generic.secrets.security.detected-google-gcm-service-account.detected-google-gcm-service-account
    mount(server, "DELETE", &format!("{base}/svac_worker"), None, json!({"service_account_id":"svac_worker","workspace_id":"wrkspc_test","type":"service_account_workspace_member_deleted"})).await;
}
