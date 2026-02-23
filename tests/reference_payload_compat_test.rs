use serde_json::json;
use threatflux::{
    CompletionResponse, InviteDeleteResponse, InviteListResponse, MessageCostReportResponse,
    MessageResponse, MessageUsageReportResponse, Skill, SkillDeleteResponse, SkillListResponse,
    SkillVersion, SkillVersionDeleteResponse, SkillVersionListResponse, StopReason, StreamEvent,
    UserDeleteResponse, UserListResponse, WorkspaceMemberDeleteResponse,
    WorkspaceMemberListResponse,
};

#[test]
fn test_messages_reference_response_payload_deserializes() {
    let response: MessageResponse = serde_json::from_value(json!({
        "id": "msg_013Zva2CMHLNnXjNJJKqJ2EF",
        "type": "message",
        "role": "assistant",
        "content": [{"type": "text", "text": "Hello!"}],
        "model": "claude-sonnet-4-5",
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 12,
            "output_tokens": 6
        },
        "created_at": "2026-02-23T00:00:00Z"
    }))
    .unwrap();

    assert_eq!(response.model, "claude-sonnet-4-5");
    assert_eq!(response.stop_reason, Some(StopReason::EndTurn));
    assert_eq!(response.usage.input_tokens, 12);
    assert_eq!(response.usage.output_tokens, 6);
}

#[test]
fn test_stop_reason_reference_values_deserialize() {
    let pause_turn: StopReason = serde_json::from_str("\"pause_turn\"").unwrap();
    let refusal: StopReason = serde_json::from_str("\"refusal\"").unwrap();

    assert_eq!(pause_turn, StopReason::PauseTurn);
    assert_eq!(refusal, StopReason::Refusal);
}

#[test]
fn test_streaming_reference_payloads_deserialize() {
    let citation_delta: StreamEvent = serde_json::from_value(json!({
        "type": "content_block_delta",
        "index": 0,
        "delta": {
            "type": "citations_delta",
            "citation": {
                "type": "char_location",
                "cited_text": "Threatflux",
                "document_index": 0,
                "start_char_index": 0,
                "end_char_index": 10
            }
        }
    }))
    .unwrap();

    match citation_delta {
        StreamEvent::ContentBlockDelta { delta, .. } => {
            assert!(delta.citation.is_some());
        }
        other => panic!("Expected ContentBlockDelta, got {other:?}"),
    }

    let message_delta: StreamEvent = serde_json::from_value(json!({
        "type": "message_delta",
        "delta": {"stop_reason": "pause_turn"},
        "usage": {
            "output_tokens": 22,
            "inference_geo": "us",
            "service_tier": "standard"
        }
    }))
    .unwrap();

    match message_delta {
        StreamEvent::MessageDelta { delta, usage } => {
            assert_eq!(delta.stop_reason, Some(StopReason::PauseTurn));
            assert_eq!(usage.output_tokens, 22);
            assert_eq!(usage.inference_geo.as_deref(), Some("us"));
            assert_eq!(usage.service_tier.as_deref(), Some("standard"));
        }
        other => panic!("Expected MessageDelta, got {other:?}"),
    }
}

#[test]
fn test_skills_reference_payloads_deserialize() {
    let list: SkillListResponse = serde_json::from_value(json!({
        "data": [
            {
                "type": "skill",
                "id": "skl_01DP8V5D1N6V3Q6N57V8Q9W0XE",
                "display_title": "Meeting notes skill",
                "created_at": "2026-02-20T16:00:00Z",
                "updated_at": "2026-02-20T16:00:00Z",
                "latest_version": "1723500000",
                "source": "custom"
            }
        ],
        "has_more": true,
        "next_page": "next_page_token"
    }))
    .unwrap();

    assert_eq!(list.data.len(), 1);
    assert_eq!(list.data[0].object_type.as_deref(), Some("skill"));
    assert_eq!(list.next_page.as_deref(), Some("next_page_token"));

    let skill: Skill = serde_json::from_value(json!({
        "type": "skill",
        "id": "skl_01DP8V5D1N6V3Q6N57V8Q9W0XE",
        "display_title": "Meeting notes skill",
        "latest_version": {
            "type": "skill_version",
            "id": "skv_01HQ8V5D1N6V3Q6N57V8Q9W0XE",
            "version": "1723500000"
        },
        "source": "custom"
    }))
    .unwrap();

    assert_eq!(skill.id, "skl_01DP8V5D1N6V3Q6N57V8Q9W0XE");

    let versions: SkillVersionListResponse = serde_json::from_value(json!({
        "data": [
            {
                "type": "skill_version",
                "id": "skv_01HQ8V5D1N6V3Q6N57V8Q9W0XE",
                "skill_id": "skl_01DP8V5D1N6V3Q6N57V8Q9W0XE",
                "version": "1723500000",
                "name": "meeting-notes",
                "description": "Summarizes notes.",
                "directory": "meeting-notes",
                "created_at": "2026-02-20T16:05:00Z"
            }
        ],
        "has_more": false
    }))
    .unwrap();

    assert_eq!(versions.data.len(), 1);
    assert_eq!(
        versions.data[0].object_type.as_deref(),
        Some("skill_version")
    );

    let version: SkillVersion = serde_json::from_value(json!({
        "type": "skill_version",
        "id": "skv_01HQ8V5D1N6V3Q6N57V8Q9W0XE",
        "skill_id": "skl_01DP8V5D1N6V3Q6N57V8Q9W0XE",
        "version": "1723500000"
    }))
    .unwrap();

    assert_eq!(
        version.skill_id.as_deref(),
        Some("skl_01DP8V5D1N6V3Q6N57V8Q9W0XE")
    );

    let deleted: SkillDeleteResponse = serde_json::from_value(json!({
        "type": "skill_deleted",
        "id": "skl_01DP8V5D1N6V3Q6N57V8Q9W0XE"
    }))
    .unwrap();
    assert_eq!(deleted.object_type.as_deref(), Some("skill_deleted"));

    let version_deleted: SkillVersionDeleteResponse = serde_json::from_value(json!({
        "type": "skill_version_deleted",
        "id": "skv_01HQ8V5D1N6V3Q6N57V8Q9W0XE"
    }))
    .unwrap();
    assert_eq!(
        version_deleted.object_type.as_deref(),
        Some("skill_version_deleted")
    );
}

#[test]
fn test_admin_reference_payloads_deserialize() {
    let users: UserListResponse = serde_json::from_value(json!({
        "data": [
            {
                "type": "user",
                "id": "user_01D8BWY8A3H31D9S7YKM4Z8Y8Q",
                "email": "admin@example.com",
                "name": "Admin User",
                "role": "admin",
                "added_at": "2026-02-20T10:00:00Z"
            }
        ],
        "has_more": false,
        "first_id": "user_01D8BWY8A3H31D9S7YKM4Z8Y8Q",
        "last_id": "user_01D8BWY8A3H31D9S7YKM4Z8Y8Q"
    }))
    .unwrap();

    assert_eq!(users.data.len(), 1);
    assert_eq!(users.data[0].object_type, "user");

    let invites: InviteListResponse = serde_json::from_value(json!({
        "data": [
            {
                "type": "invite",
                "id": "inv_01D8BWY8A3H31D9S7YKM4Z8Y8Q",
                "email": "newuser@example.com",
                "role": "developer",
                "status": "pending",
                "invited_at": "2026-02-20T10:15:00Z",
                "expires_at": "2026-02-27T10:15:00Z"
            }
        ],
        "has_more": false
    }))
    .unwrap();

    assert_eq!(invites.data.len(), 1);
    assert_eq!(invites.data[0].object_type, "invite");

    let members: WorkspaceMemberListResponse = serde_json::from_value(json!({
        "data": [
            {
                "type": "workspace_member",
                "workspace_id": "ws_01D8BWY8A3H31D9S7YKM4Z8Y8Q",
                "user_id": "user_01D8BWY8A3H31D9S7YKM4Z8Y8Q",
                "workspace_role": "workspace_developer"
            }
        ],
        "has_more": false
    }))
    .unwrap();

    assert_eq!(members.data.len(), 1);
    assert_eq!(members.data[0].object_type, "workspace_member");

    let user_deleted: UserDeleteResponse = serde_json::from_value(json!({
        "type": "user_deleted",
        "id": "user_01D8BWY8A3H31D9S7YKM4Z8Y8Q"
    }))
    .unwrap();
    assert_eq!(user_deleted.object_type, "user_deleted");

    let invite_deleted: InviteDeleteResponse = serde_json::from_value(json!({
        "type": "invite_deleted",
        "id": "inv_01D8BWY8A3H31D9S7YKM4Z8Y8Q"
    }))
    .unwrap();
    assert_eq!(invite_deleted.object_type, "invite_deleted");

    let member_deleted: WorkspaceMemberDeleteResponse = serde_json::from_value(json!({
        "type": "workspace_member_deleted",
        "workspace_id": "ws_01D8BWY8A3H31D9S7YKM4Z8Y8Q",
        "user_id": "user_01D8BWY8A3H31D9S7YKM4Z8Y8Q"
    }))
    .unwrap();
    assert_eq!(member_deleted.object_type, "workspace_member_deleted");
}

#[test]
fn test_message_usage_report_reference_payload_deserializes() {
    let report: MessageUsageReportResponse = serde_json::from_value(json!({
        "data": [
            {
                "starting_at": "2026-02-20T00:00:00Z",
                "ending_at": "2026-02-21T00:00:00Z",
                "results": [
                    {
                        "model": "claude-sonnet-4-5",
                        "workspace_id": "ws_01D8BWY8A3H31D9S7YKM4Z8Y8Q",
                        "api_key_id": "sk-ant-admin-test",
                        "service_tier": "standard",
                        "context_window": "200k",
                        "uncached_input_tokens": 120,
                        "cached_input_tokens": 30,
                        "cache_creation_input_tokens": 10,
                        "output_tokens": 55,
                        "requests": 3
                    }
                ]
            }
        ],
        "has_more": false,
        "next_page": null
    }))
    .unwrap();

    assert_eq!(report.data.len(), 1);
    assert!(report.data[0].starting_at.is_some());

    let results = report.data[0]
        .extra
        .get("results")
        .and_then(|v| v.as_array())
        .unwrap();
    assert_eq!(results[0]["service_tier"], "standard");
    assert_eq!(results[0]["requests"], 3);
}

#[test]
fn test_message_cost_report_reference_payload_deserializes() {
    let report: MessageCostReportResponse = serde_json::from_value(json!({
        "data": [
            {
                "starting_at": "2026-02-20T00:00:00Z",
                "ending_at": "2026-02-21T00:00:00Z",
                "results": [
                    {
                        "workspace_id": "ws_01D8BWY8A3H31D9S7YKM4Z8Y8Q",
                        "service_tier": "standard",
                        "cost": {
                            "currency": "USD",
                            "amount": "12.34"
                        }
                    }
                ]
            }
        ],
        "has_more": false,
        "next_page": null
    }))
    .unwrap();

    assert_eq!(report.data.len(), 1);
    let results = report.data[0]
        .extra
        .get("results")
        .and_then(|v| v.as_array())
        .unwrap();
    assert_eq!(results[0]["cost"]["currency"], "USD");
    assert_eq!(results[0]["cost"]["amount"], "12.34");
}

#[test]
fn test_legacy_completions_reference_payload_deserializes() {
    let completion: CompletionResponse = serde_json::from_value(json!({
        "id": "compl_018CKm6gsux7P8yMcwZbeCPw",
        "type": "completion",
        "completion": " Hello! My name is Claude.",
        "model": "claude-2.1",
        "stop_reason": "stop_sequence",
        "stop": "\n\nHuman:"
    }))
    .unwrap();

    assert_eq!(completion.object_type, "completion");
    assert_eq!(completion.model, "claude-2.1");
    assert_eq!(completion.stop, Some("\n\nHuman:".to_string()));
}
