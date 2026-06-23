//! Integration tests for the Managed Agents sub-resource APIs
//! (beta: managed-agents-2026-04-01).
//!
//! Split from `managed_agents_test.rs` / `managed_agents_more_test.rs` to keep
//! each file within the per-file size limit. Covers session resources & threads,
//! session update, and the ergonomic session-event send helpers. The remaining
//! sub-resource coverage (memories, credentials, deployment runs, and the
//! get/update endpoints) lives in `managed_agents_sub2_test.rs`. Same
//! conventions throughout: a `wiremock` mock server, assertions on HTTP
//! method/path, and that the `anthropic-beta` header contains the managed-agents
//! beta value.

use serde_json::json;
use threatflux_anthropic_sdk::{
    models::managed_agents::{
        SessionResourceSpec, SessionResourceUpdateRequest, SessionUpdateRequest,
    },
    types::Pagination,
    Client, Config,
};
use wiremock::{
    matchers::{header_regex, method, path, query_param},
    Mock, MockServer, ResponseTemplate,
};

use crate::common::fixtures;

#[cfg(test)]
mod managed_agents_sub_tests {
    use super::*;

    async fn setup_test_client(mock_server: &MockServer) -> Client {
        let config = Config::new("sk-ant-test-key")
            .unwrap()
            .with_base_url(mock_server.uri().parse().unwrap());
        Client::new(config)
    }

    /// Matcher asserting the comma-joined `anthropic-beta` header contains the
    /// managed-agents beta value.
    fn managed_agents_beta() -> impl wiremock::Match {
        header_regex("anthropic-beta", "managed-agents-2026-04-01")
    }

    // -------------------------------------------------------------------------
    // Session resources
    //
    // NOTE: `SessionResource` flattens an internally-tagged `SessionResourceSpec`
    // (`#[serde(tag = "type")]`) next to its own renamed `type` field, a serde
    // combination that does not deserialize. So for `add`/`list`/`get`/`update`
    // we verify the request is correctly DISPATCHED (method + path + beta header,
    // via `mount_as_scoped` + `.expect(1)` verified on drop) rather than asserting
    // the response decodes. `delete` returns `()` and is asserted normally.
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_session_resource_add() {
        let mock_server = MockServer::start().await;

        let guard = Mock::given(method("POST"))
            .and(path("/v1/sessions/session_test123/resources"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(1)
            .mount_as_scoped(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let spec = SessionResourceSpec::File {
            file_id: "file_test123".to_string(),
            mount_path: Some("/in".to_string()),
        };
        let _ = client
            .sessions()
            .resources("session_test123")
            .add(spec, None)
            .await;
        // Scoped guard verifies on drop that the POST was dispatched.
        drop(guard);
    }

    #[tokio::test]
    async fn test_session_resource_list_pagination() {
        let mock_server = MockServer::start().await;

        let guard = Mock::given(method("GET"))
            .and(path("/v1/sessions/session_test123/resources"))
            .and(query_param("limit", "5"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [],
                "has_more": false,
                "first_id": null,
                "last_id": null
            })))
            .expect(1)
            .mount_as_scoped(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let list = client
            .sessions()
            .resources("session_test123")
            .list(Some(Pagination::new().with_limit(5)), None)
            .await;
        // Empty data decodes fine, so this path also round-trips.
        assert!(list.is_ok(), "resource list should succeed: {list:?}");
        assert_eq!(list.unwrap().data.len(), 0);
        drop(guard);
    }

    #[tokio::test]
    async fn test_session_resource_get() {
        let mock_server = MockServer::start().await;

        let guard = Mock::given(method("GET"))
            .and(path("/v1/sessions/session_test123/resources/res_test123"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(1)
            .mount_as_scoped(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let _ = client
            .sessions()
            .resources("session_test123")
            .get("res_test123", None)
            .await;
        drop(guard);
    }

    #[tokio::test]
    async fn test_session_resource_update() {
        let mock_server = MockServer::start().await;

        // Update is a POST to the resource path (not PATCH).
        let guard = Mock::given(method("POST"))
            .and(path("/v1/sessions/session_test123/resources/res_test123"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(1)
            .mount_as_scoped(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let request = SessionResourceUpdateRequest::new();
        let _ = client
            .sessions()
            .resources("session_test123")
            .update("res_test123", request, None)
            .await;
        drop(guard);
    }

    #[tokio::test]
    async fn test_session_resource_delete() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/v1/sessions/session_test123/resources/res_test123"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"deleted": true})))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let result = client
            .sessions()
            .resources("session_test123")
            .delete("res_test123", None)
            .await;
        assert!(result.is_ok(), "resource delete should succeed: {result:?}");
    }

    // -------------------------------------------------------------------------
    // Session threads
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_session_thread_list_pagination() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/sessions/session_test123/threads"))
            .and(query_param("limit", "2"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [fixtures::test_session_thread()],
                "has_more": false,
                "first_id": "thread_test123",
                "last_id": "thread_test123"
            })))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let list = client
            .sessions()
            .threads("session_test123")
            .list(Some(Pagination::new().with_limit(2)), None)
            .await;
        assert!(list.is_ok(), "thread list should succeed: {list:?}");
        assert_eq!(list.unwrap().data.len(), 1);
    }

    #[tokio::test]
    async fn test_session_thread_get() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/sessions/session_test123/threads/thread_test123"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_session_thread()))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let thread = client
            .sessions()
            .threads("session_test123")
            .get("thread_test123", None)
            .await;
        assert!(thread.is_ok(), "thread get should succeed: {thread:?}");
        assert_eq!(thread.unwrap().id, "thread_test123");
    }

    #[tokio::test]
    async fn test_session_thread_archive() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path(
                "/v1/sessions/session_test123/threads/thread_test123/archive",
            ))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_session_thread()))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let thread = client
            .sessions()
            .threads("session_test123")
            .archive("thread_test123", None)
            .await;
        assert!(thread.is_ok(), "thread archive should succeed: {thread:?}");
    }

    #[tokio::test]
    async fn test_session_thread_list_events() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path(
                "/v1/sessions/session_test123/threads/thread_test123/events",
            ))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [{
                    "type": "agent.message",
                    "id": "evt_1",
                    "processed_at": "2026-04-01T00:00:00Z",
                    "content": [{"type": "text", "text": "hi"}]
                }],
                "has_more": false,
                "first_id": "evt_1",
                "last_id": "evt_1"
            })))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let list = client
            .sessions()
            .threads("session_test123")
            .list_events("thread_test123", None, None)
            .await;
        assert!(list.is_ok(), "thread list_events should succeed: {list:?}");
        assert_eq!(list.unwrap().data.len(), 1);
    }

    #[tokio::test]
    async fn test_session_thread_stream_events() {
        use futures::StreamExt;

        let mock_server = MockServer::start().await;

        let stream_body = [
            "event: agent.message\n",
            "data: {\"type\":\"agent.message\",\"id\":\"evt_1\",\"processed_at\":\"2026-04-01T00:00:00Z\",\"content\":[{\"type\":\"text\",\"text\":\"hi\"}]}\n",
            "\n",
        ]
        .join("");

        Mock::given(method("GET"))
            .and(path(
                "/v1/sessions/session_test123/threads/thread_test123/events/stream",
            ))
            .and(managed_agents_beta())
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(stream_body),
            )
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let stream = client
            .sessions()
            .threads("session_test123")
            .stream_events("thread_test123", None)
            .await;
        assert!(
            stream.is_ok(),
            "thread event stream should open: {:?}",
            stream.as_ref().err()
        );
        let events: Vec<_> = stream.unwrap().collect().await;
        assert_eq!(events.len(), 1, "should decode one event");
        assert!(events.iter().all(|e| e.is_ok()), "all events should decode");
    }

    // -------------------------------------------------------------------------
    // Session update + event helpers
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_session_update() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/sessions/session_test123"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_session()))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let request = SessionUpdateRequest::new().title("renamed");
        let session = client
            .sessions()
            .update("session_test123", request, None)
            .await;
        assert!(
            session.is_ok(),
            "session update should succeed: {session:?}"
        );
    }

    /// Drives every ergonomic send helper (interrupt/confirm_tool/
    /// custom_tool_result/define_outcome/system_message). They all POST to the
    /// same events path, so one mounted mock serves the whole sequence.
    #[tokio::test]
    async fn test_session_event_send_helpers() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/sessions/session_test123/events"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "type": "agent.message",
                "id": "evt_ack",
                "processed_at": "2026-04-01T00:00:00Z",
                "content": [{"type": "text", "text": "ack"}]
            })))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let events = client.sessions().events("session_test123");

        assert!(
            events.interrupt(None).await.is_ok(),
            "interrupt should send"
        );
        assert!(
            events.confirm_tool("tu_1", true, None).await.is_ok(),
            "confirm_tool should send"
        );
        assert!(
            events
                .custom_tool_result("tu_1", json!({"ok": true}), None)
                .await
                .is_ok(),
            "custom_tool_result should send"
        );
        assert!(
            events
                .define_outcome(json!({"status": "done"}), None)
                .await
                .is_ok(),
            "define_outcome should send"
        );
        assert!(
            events.system_message("be terse", None).await.is_ok(),
            "system_message should send"
        );
    }
}
