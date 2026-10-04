//! Current Files metadata and upload contracts.

use super::*;

#[tokio::test]
async fn current_files_parse_upload_get_list_and_download_contract() {
    let server = MockServer::start().await;
    mount_current_files(&server).await;
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

async fn mount_current_files(server: &MockServer) {
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
            .mount(server)
            .await;
    }
    Mock::given(method("GET"))
        .and(path("/v1/files/file_current/content"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"download".to_vec()))
        .expect(1)
        .mount(server)
        .await;
}
