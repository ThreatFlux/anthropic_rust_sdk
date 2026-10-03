//! Current resource schemas, bounded traversal, and genuinely incremental batch results.
//! Fixtures follow official Python SDK revision 18f25547f20cf5f01da69ac611e700e3bc9ebf21.

use futures::StreamExt;
use serde_json::{json, Value};
use std::time::Duration;
use threatflux_anthropic_sdk::{
    api::message_batches::BatchResultsStreamOptions,
    models::{
        file::{File, FileListParams, FileUploadRequest},
        skill::{
            CurrentSkillCreateRequest, CurrentSkillVersionCreateRequest, SkillFileUpload,
            SkillListParams, SkillVersionListParams,
        },
    },
    types::{Pagination, PaginationLimits, RequestOptions},
    Client, Config,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use wiremock::{
    matchers::{header, method, path, query_param},
    Mock, MockServer, ResponseTemplate,
};

fn client(server: &MockServer) -> Client {
    Client::new(
        Config::new("sk-ant-test-key")
            .unwrap()
            .with_base_url(server.uri().parse().unwrap()),
    )
}

fn file(id: &str) -> Value {
    json!({"id":id,"type":"file","created_at":"2026-10-03T00:00:00Z","filename":"document.txt","mime_type":"text/plain","size_bytes":12,"downloadable":true,"expires_at":"2026-10-04T00:00:00Z","future_metadata":{"nested":7}})
}

fn skill(id: &str) -> Value {
    json!({"id":id,"type":"skill","created_at":"2026-10-03T00:00:00Z","updated_at":"2026-10-03T00:00:00Z","display_name":"My Skill","latest_version_id":"skv_current","source":{"type":"custom","future_source":true},"future_skill":[1,2]})
}

fn version(id: &str) -> Value {
    json!({"id":id,"type":"skill_version","created_at":"2026-10-03T00:00:00Z","description":"Use this skill","name":"my-skill","skill_id":"skl_current","future_version":3})
}

fn upload_file() -> SkillFileUpload {
    SkillFileUpload::new(
        "my-skill/SKILL.md",
        b"---\nname: my-skill\n---\n".to_vec(),
        "text/markdown",
    )
}

#[tokio::test]
async fn current_files_parse_upload_get_list_and_download_contract() {
    let server = MockServer::start().await;
    for (verb, endpoint, payload) in [
        ("POST", "/v1/files", file("file_current")),
        ("GET", "/v1/files/file_current", file("file_current")),
        (
            "GET",
            "/v1/files",
            json!({"data":[file("file_current")],"next_page":null}),
        ),
    ] {
        Mock::given(method(verb))
            .and(path(endpoint))
            .and(header("x-api-key", "sk-ant-test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(payload))
            .expect(1)
            .mount(&server)
            .await;
    }
    Mock::given(method("GET"))
        .and(path("/v1/files/file_current/content"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"download".to_vec()))
        .expect(1)
        .mount(&server)
        .await;
    let api = client(&server).files();
    let uploaded = api
        .upload(
            FileUploadRequest::new(b"data".to_vec(), "document.txt", "text/plain")
                .expires_in_seconds(3600),
            None,
        )
        .await
        .unwrap()
        .file;
    assert_eq!(uploaded.purpose, None);
    assert_eq!(uploaded.downloadable, Some(true));
    assert!(uploaded.expires_at.is_some());
    assert_eq!(uploaded.extra["future_metadata"], json!({"nested":7}));
    assert_eq!(api.get("file_current", None).await.unwrap(), uploaded);
    assert_eq!(api.list(None, None).await.unwrap().data, vec![uploaded]);
    assert_eq!(
        api.download("file_current", None).await.unwrap(),
        b"download"
    );
    let requests = server.received_requests().await.unwrap();
    let upload = &requests[0];
    let body = String::from_utf8(upload.body.clone()).unwrap();
    assert!(body.contains("name=\"expires_in_seconds\""));
    assert!(!body.contains("name=\"purpose\""));
    assert!(upload.headers["content-type"]
        .to_str()
        .unwrap()
        .contains("boundary="));
    let omitted: File = serde_json::from_value(json!({"id":"f","type":"file","created_at":"2026-10-03T00:00:00Z","filename":"x","mime_type":"text/plain","size_bytes":0})).unwrap();
    assert_eq!(omitted.downloadable, None);
}

#[tokio::test]
async fn file_expiration_and_lookup_validation_never_request_invalid_input() {
    let server = MockServer::start().await;
    let api = client(&server).files();
    for seconds in [0, 3599, 7_776_001, u32::MAX] {
        assert!(api
            .upload(
                FileUploadRequest::new(vec![1], "x", "text/plain").expires_in_seconds(seconds),
                None
            )
            .await
            .is_err());
    }
    for seconds in [3600, 7_776_000] {
        assert!(FileUploadRequest::new(vec![1], "x", "text/plain")
            .expires_in_seconds(seconds)
            .validate()
            .is_ok());
    }
    assert!(api
        .list_with_params(
            None,
            FileListParams::new()
                .with_ids(vec!["f".into()])
                .with_limit(1),
            None
        )
        .await
        .is_err());
    assert!(api
        .list(Some(Pagination::new().with_after("f")), None)
        .await
        .is_err());
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn current_skill_lifecycle_uses_name_object_source_and_version_ids() {
    let server = MockServer::start().await;
    for (verb, endpoint, payload) in [
        ("POST", "/v1/skills", skill("skl_current")),
        ("GET", "/v1/skills/skl_current", skill("skl_current")),
        (
            "GET",
            "/v1/skills",
            json!({"data":[skill("skl_current")],"next_page":null}),
        ),
        (
            "POST",
            "/v1/skills/skl_current/versions",
            version("skv_current"),
        ),
        (
            "GET",
            "/v1/skills/skl_current/versions/skv_current",
            version("skv_current"),
        ),
        (
            "GET",
            "/v1/skills/skl_current/versions",
            json!({"data":[version("skv_current")],"next_page":null}),
        ),
        (
            "DELETE",
            "/v1/skills/skl_current/versions/skv_current",
            json!({"id":"skv_current","type":"skill_version_deleted"}),
        ),
        (
            "DELETE",
            "/v1/skills/skl_current",
            json!({"id":"skl_current","type":"skill_deleted"}),
        ),
    ] {
        Mock::given(method(verb))
            .and(path(endpoint))
            .and(header("x-api-key", "sk-ant-test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(payload))
            .expect(1)
            .mount(&server)
            .await;
    }
    let api = client(&server).skills_current();
    let created = api
        .create(
            CurrentSkillCreateRequest::new()
                .display_name("My Skill")
                .add_file(upload_file()),
            Some(
                RequestOptions::new()
                    .with_beta_feature("unrelated-2026-01-01")
                    .with_header("anthropic-workspace-id", "workspace_test"),
            ),
        )
        .await
        .unwrap();
    assert_eq!(created.source.source_type, "custom");
    assert_eq!(created.source.extra["future_source"], true);
    assert_eq!(created.latest_version_id, "skv_current");
    assert_eq!(api.get("skl_current", None).await.unwrap(), created);
    assert_eq!(api.list(None, None).await.unwrap().data, vec![created]);
    let uploaded = api
        .create_version(
            "skl_current",
            CurrentSkillVersionCreateRequest::new().add_file(upload_file()),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        api.get_version("skl_current", "skv_current", None)
            .await
            .unwrap(),
        uploaded
    );
    assert_eq!(
        api.list_versions("skl_current", None, None)
            .await
            .unwrap()
            .data,
        vec![uploaded]
    );
    api.delete_version("skl_current", "skv_current", None)
        .await
        .unwrap();
    api.delete("skl_current", None).await.unwrap();
    let requests = server.received_requests().await.unwrap();
    let form = String::from_utf8(requests[0].body.clone()).unwrap();
    assert!(form.contains("name=\"display_name\""));
    assert!(!form.contains("name=\"display_title\""));
    assert_eq!(
        requests[0].headers["anthropic-beta"],
        "unrelated-2026-01-01"
    );
    assert_eq!(
        requests[0].headers["anthropic-workspace-id"],
        "workspace_test"
    );
    assert!(requests.iter().all(|request| request
        .headers
        .get("anthropic-beta")
        .is_none_or(|value| !value.to_str().unwrap().contains("skills-2025-10-02"))));
}

#[tokio::test]
async fn current_skill_conflicting_beta_and_unsafe_layout_fail_locally() {
    let server = MockServer::start().await;
    let api = client(&server).skills_current();
    for options in [
        RequestOptions::new().with_skills_api(),
        RequestOptions::new().with_beta_feature("skills-2025-10-02"),
        RequestOptions::new().with_header("AnThRoPiC-BeTa", "other, skills-2025-10-02"),
    ] {
        assert!(api.list(None, Some(options)).await.is_err());
    }
    for name in [
        "../SKILL.md",
        "/root/SKILL.md",
        "one/../SKILL.md",
        "one\\SKILL.md",
        "one/readme.md",
    ] {
        let request = CurrentSkillCreateRequest::new().add_file(SkillFileUpload::new(
            name,
            vec![1],
            "text/plain",
        ));
        assert!(api.create(request, None).await.is_err());
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn token_pages_preserve_filters_headers_and_continue_without_has_more() {
    let server = MockServer::start().await;
    for (cursor, next, id) in [
        (None, Some("page one&two"), "f1"),
        (Some("page one&two"), Some("page_three"), "f2"),
        (Some("page_three"), None, "f3"),
    ] {
        let mut mock = Mock::given(method("GET"))
            .and(path("/v1/files"))
            .and(query_param("scope_id", "session & one"))
            .and(query_param("limit", "7"))
            .and(header("test-traversal", "preserved"));
        if let Some(cursor) = cursor {
            mock = mock.and(query_param("page", cursor));
        } else {
            mock = mock.and(|request: &wiremock::Request| {
                request.url.query_pairs().all(|(key, _)| key != "page")
            });
        }
        mock.respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"data":[file(id)],"next_page":next})),
        )
        .expect(1)
        .mount(&server)
        .await;
    }
    let files = client(&server)
        .files()
        .list_all_with_limits(
            FileListParams::new()
                .scope_id("session & one")
                .with_limit(7),
            PaginationLimits::default(),
            Some(
                RequestOptions::new()
                    .with_header("test-traversal", "preserved")
                    .with_files_api()
                    .with_timeout(Duration::from_secs(1))
                    .no_retry(),
            ),
        )
        .await
        .unwrap();
    assert_eq!(
        files
            .iter()
            .map(|file| file.id.as_str())
            .collect::<Vec<_>>(),
        ["f1", "f2", "f3"]
    );
}

#[tokio::test]
async fn current_skill_and_version_token_traversal() {
    let server = MockServer::start().await;
    for (endpoint, first, second) in [
        ("/v1/skills", skill("s1"), skill("s2")),
        (
            "/v1/skills/skl_current/versions",
            version("v1"),
            version("v2"),
        ),
    ] {
        Mock::given(method("GET"))
            .and(path(endpoint))
            .and(query_param("page", "page_next"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"data":[second],"next_page":null})),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(endpoint))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"data":[first],"next_page":"page_next"})),
            )
            .expect(1)
            .mount(&server)
            .await;
    }
    let api = client(&server).skills_current();
    assert_eq!(
        api.list_all_with_limits(SkillListParams::new(), PaginationLimits::default(), None)
            .await
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        api.list_all_versions_with_limits(
            "skl_current",
            SkillVersionListParams::new(),
            PaginationLimits::default(),
            None
        )
        .await
        .unwrap()
        .len(),
        2
    );
}

#[tokio::test]
async fn model_id_pages_encode_cursors_and_preserve_options() {
    let server = MockServer::start().await;
    for (cursor, next, id, more) in [
        (None, "cursor & one", "m1", true),
        (Some("cursor & one"), "last", "m2", true),
        (Some("last"), "m3", "m3", false),
    ] {
        let mut mock = Mock::given(method("GET"))
            .and(path("/v1/models"))
            .and(query_param("limit", "5"))
            .and(header("test-options", "every-page"));
        if let Some(cursor) = cursor {
            mock = mock.and(query_param("after", cursor));
        } else {
            mock = mock.and(|request: &wiremock::Request| {
                request.url.query_pairs().all(|(key, _)| key != "after")
            });
        }
        mock.respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data":[{"id":id,"type":"model","display_name":"Model","created_at":"2026-10-03T00:00:00Z"}],
            "has_more":more,"first_id":id,"last_id":next
        }))).expect(1).mount(&server).await;
    }
    let models = client(&server)
        .models()
        .list_all_with_limits(
            Pagination::new().with_limit(5),
            PaginationLimits::new(3, 3).unwrap(),
            Some(
                RequestOptions::new()
                    .with_header("test-options", "every-page")
                    .no_retry(),
            ),
        )
        .await
        .unwrap();
    assert_eq!(
        models
            .iter()
            .map(|model| model.id.as_str())
            .collect::<Vec<_>>(),
        ["m1", "m2", "m3"]
    );
}

#[tokio::test]
async fn model_invalid_continuation_and_reverse_contract() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/v1/models")).and(query_param("before", "start"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data":[{"id":"model","type":"model","display_name":"Model","created_at":"2026-10-03T00:00:00Z"}],
            "has_more":true,"first_id":null,"last_id":"must-not-be-used"
        }))).expect(1).mount(&server).await;
    assert!(client(&server)
        .models()
        .list_all_with_limits(
            Pagination::new().with_before("start"),
            PaginationLimits::default(),
            None
        )
        .await
        .is_err());
}

async fn write_chunk(socket: &mut tokio::net::TcpStream, payload: &[u8]) {
    socket
        .write_all(format!("{:X}\r\n", payload.len()).as_bytes())
        .await
        .unwrap();
    socket.write_all(payload).await.unwrap();
    socket.write_all(b"\r\n").await.unwrap();
    socket.flush().await.unwrap();
}

async fn read_request_headers(socket: &mut tokio::net::TcpStream) {
    let mut headers = Vec::new();
    let mut buffer = [0_u8; 1024];
    while !headers.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
        let count = socket.read(&mut buffer).await.unwrap();
        assert!(count > 0, "connection closed before request headers");
        headers.extend_from_slice(&buffer[..count]);
        assert!(headers.len() <= 8192, "test request headers exceeded bound");
    }
}

#[tokio::test]
async fn batch_results_yield_before_download_finishes_and_handle_utf8_final_row() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (continue_tx, continue_rx) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        read_request_headers(&mut socket).await;
        socket.write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nContent-Type: application/jsonl\r\nConnection: close\r\n\r\n").await.unwrap();
        let first = "{\"custom_id\":\"one\",\"result\":{\"type\":\"canceled\"}}\r\n";
        write_chunk(&mut socket, first.as_bytes()).await;
        continue_rx.await.unwrap();
        let second = "\n{\"custom_id\":\"日本語\",\"result\":{\"type\":\"expired\"}}";
        for chunk in second.as_bytes().chunks(2) {
            write_chunk(&mut socket, chunk).await;
        }
        socket.write_all(b"0\r\n\r\n").await.unwrap();
    });
    let api = Client::new(
        Config::new("sk-ant-test")
            .unwrap()
            .with_base_url(format!("http://{address}").parse().unwrap()),
    )
    .message_batches();
    let mut stream = api.results_stream("batch_test", None).await.unwrap();
    let first = tokio::time::timeout(Duration::from_secs(1), stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(first.custom_id, "one");
    continue_tx.send(()).unwrap();
    assert_eq!(stream.next().await.unwrap().unwrap().custom_id, "日本語");
    assert!(stream.next().await.is_none());
    server.await.unwrap();
}

#[tokio::test]
async fn batch_jsonl_errors_once_without_exposing_rows_and_respects_limits() {
    for (body, limit) in [
        ("{\"custom_id\":\"valid\",\"result\":{\"type\":\"canceled\"}}\nprivate-secret-invalid\n{}", 1024),
        ("xxxxxxxxxxxxxxxxxxxx", 8),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(path("/v1/messages/batches/b/results"))
            .respond_with(ResponseTemplate::new(200).set_body_string(body)).mount(&server).await;
        let mut stream = client(&server).message_batches().results_stream_with_options("b", BatchResultsStreamOptions::new(limit).unwrap(), None).await.unwrap();
        if limit == 1024 { assert_eq!(stream.next().await.unwrap().unwrap().custom_id, "valid"); }
        let error = stream.next().await.unwrap().unwrap_err().to_string();
        assert!(error.contains("row"));
        assert!(!error.contains("private-secret"));
        assert!(stream.next().await.is_none());
    }
}

#[tokio::test]
async fn dropping_batch_stream_closes_unfinished_http_response() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        read_request_headers(&mut socket).await;
        socket
            .write_all(
                b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
            )
            .await
            .unwrap();
        write_chunk(
            &mut socket,
            b"{\"custom_id\":\"one\",\"result\":{\"type\":\"canceled\"}}\n",
        )
        .await;
        let mut byte = [0];
        // An incomplete HTTP/1 response cannot return to the connection pool.
        tokio::time::timeout(Duration::from_secs(2), socket.read(&mut byte))
            .await
            .unwrap()
            .unwrap()
    });
    let api = Client::new(
        Config::new("sk-ant-test")
            .unwrap()
            .with_base_url(format!("http://{address}").parse().unwrap()),
    )
    .message_batches();
    let mut stream = api.results_stream("batch", None).await.unwrap();
    stream.next().await.unwrap().unwrap();
    drop(stream);
    assert_eq!(server.await.unwrap(), 0);
}

#[tokio::test]
async fn interrupted_batch_transport_fails_once() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        read_request_headers(&mut socket).await;
        socket.write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\nA\r\nxx").await.unwrap();
    });
    let api = Client::new(
        Config::new("sk-ant-test")
            .unwrap()
            .with_base_url(format!("http://{address}").parse().unwrap()),
    )
    .message_batches();
    let mut stream = api.results_stream("batch", None).await.unwrap();
    assert!(stream
        .next()
        .await
        .unwrap()
        .unwrap_err()
        .to_string()
        .contains("Transport failure reading batch result row 1"));
    assert!(stream.next().await.is_none());
    server.await.unwrap();
}
