//! Integration tests for Files API
//!
//! Tests Files API operations with mocked responses.

use wiremock::{MockServer, Mock, ResponseTemplate, matchers::{method, path, header, query_param}};
use threatflux::{Client, Config, models::file::FileUploadRequest, types::Pagination};
use serde_json::json;
use pretty_assertions::assert_eq;

mod common;
use crate::common::{fixtures, mock_server};

#[cfg(test)]
mod files_api_tests {
    use super::*;
    
    async fn setup_test_client(mock_server: &MockServer) -> Client {
        let config = Config::new("test-key")
            .unwrap()
            .with_base_url(mock_server.uri().parse().unwrap());
        Client::new(config)
    }

    #[tokio::test]
    async fn test_upload_file_success() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/v1/files"))
            .and(header("x-api-key", "test-key"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_file_upload_response()))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let content = b"Hello, this is test file content for the Threatflux SDK!";
        let upload_request = FileUploadRequest::new(
            content.to_vec(),
            "test.txt",
            "text/plain"
        ).purpose("user_data");
        
        let response = client.files().upload(upload_request, None).await;
        
        assert!(response.is_ok());
        let upload_result = response.unwrap();
        assert_eq!(upload_result.file.filename, "test.txt");
        assert_eq!(upload_result.file.mime_type, "text/plain");
        assert_eq!(upload_result.file.size_bytes, 1024); // From fixture
    }

    #[tokio::test]
    async fn test_upload_different_file_types() {
        let mock_server = MockServer::start().await;
        
        // Test different file types
        let test_files = vec![
            ("document.pdf", "application/pdf", b"PDF content".to_vec()),
            ("image.png", "image/png", b"PNG content".to_vec()),
            ("data.json", "application/json", b"{\"test\": true}".as_bytes().to_vec()),
            ("script.py", "text/x-python", b"print('hello')".to_vec()),
        ];
        
        for (filename, mime_type, _content) in &test_files {
            let mut file_response = fixtures::test_file();
            file_response.filename = filename.to_string();
            file_response.mime_type = mime_type.to_string();
            
            let upload_response = threatflux::models::file::FileUploadResponse {
                file: file_response,
            };
            
            Mock::given(method("POST"))
                .and(path("/v1/files"))
                .respond_with(ResponseTemplate::new(200)
                    .set_body_json(&upload_response))
                .mount(&mock_server)
                .await;
        }
        
        let client = setup_test_client(&mock_server).await;
        
        for (filename, mime_type, content) in test_files {
            let upload_request = FileUploadRequest::new(
                content,
                &filename,
                &mime_type
            ).purpose("user_data");
            
            let response = client.files().upload(upload_request, None).await;
            assert!(response.is_ok());
            
            let upload_result = response.unwrap();
            assert_eq!(upload_result.file.filename, filename);
            assert_eq!(upload_result.file.mime_type, mime_type);
        }
    }

    #[tokio::test]
    async fn test_list_files() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/v1/files"))
            .and(header("x-api-key", "test-key"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_file_list_response()))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let response = client.files().list(None, None).await;
        
        assert!(response.is_ok());
        let files = response.unwrap();
        assert_eq!(files.object, "list");
        assert!(!files.data.is_empty());
        assert_eq!(files.data[0].filename, "test.txt");
    }

    #[tokio::test]
    async fn test_list_files_with_pagination() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/v1/files"))
            .and(query_param("limit", "5"))
            .and(query_param("after", "file_cursor"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_file_list_response()))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let pagination = Pagination::new()
            .with_limit(5)
            .with_after("file_cursor");
        
        let response = client.files().list(Some(pagination), None).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_get_file() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/v1/files/file_test123"))
            .and(header("x-api-key", "test-key"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_file()))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let response = client.files().get("file_test123", None).await;
        
        assert!(response.is_ok());
        let file = response.unwrap();
        assert_eq!(file.id, "file_test123");
        assert_eq!(file.filename, "test.txt");
        assert_eq!(file.status, threatflux::models::file::FileStatus::Uploaded);
    }

    #[tokio::test]
    async fn test_download_file() {
        let mock_server = MockServer::start().await;
        
        let file_content = b"This is the downloaded file content";
        
        Mock::given(method("GET"))
            .and(path("/v1/files/file_test123/content"))
            .and(header("x-api-key", "test-key"))
            .respond_with(ResponseTemplate::new(200)
                .set_header("content-type", "text/plain")
                .set_header("content-disposition", "attachment; filename=\"test.txt\"")
                .set_body_bytes(file_content))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let response = client.files().download("file_test123", None).await;
        
        assert!(response.is_ok());
        let download = response.unwrap();
        assert_eq!(download.content, file_content);
        assert_eq!(download.filename, "test.txt");
        assert_eq!(download.mime_type, "text/plain");
    }

    #[tokio::test]
    async fn test_delete_file() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("DELETE"))
            .and(path("/v1/files/file_test123"))
            .and(header("x-api-key", "test-key"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&json!({"deleted": true})))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let response = client.files().delete("file_test123", None).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_file_status_transitions() {
        let mock_server = MockServer::start().await;
        
        // Test different file statuses
        let statuses = vec![
            threatflux::models::file::FileStatus::Uploaded,
            threatflux::models::file::FileStatus::Processing,
            threatflux::models::file::FileStatus::Processed,
            threatflux::models::file::FileStatus::Error,
        ];
        
        for (i, status) in statuses.iter().enumerate() {
            let mut file = fixtures::test_file();
            file.id = format!("file_{}", i);
            file.status = status.clone();
            
            Mock::given(method("GET"))
                .and(path(&format!("/v1/files/file_{}", i)))
                .respond_with(ResponseTemplate::new(200)
                    .set_body_json(&file))
                .mount(&mock_server)
                .await;
        }
        
        let client = setup_test_client(&mock_server).await;
        
        for (i, expected_status) in statuses.iter().enumerate() {
            let response = client.files().get(&format!("file_{}", i), None).await;
            assert!(response.is_ok());
            
            let file = response.unwrap();
            assert_eq!(file.status, *expected_status);
        }
    }

    #[tokio::test]
    async fn test_file_purposes() {
        let mock_server = MockServer::start().await;
        
        let purposes = vec![
            ("user_data", threatflux::models::file::FilePurpose::UserData),
            ("assistant_data", threatflux::models::file::FilePurpose::AssistantData),
        ];
        
        for (purpose_str, purpose_enum) in purposes {
            let mut file = fixtures::test_file();
            file.purpose = purpose_enum.clone();
            
            let upload_response = threatflux::models::file::FileUploadResponse {
                file: file.clone(),
            };
            
            Mock::given(method("POST"))
                .and(path("/v1/files"))
                .respond_with(ResponseTemplate::new(200)
                    .set_body_json(&upload_response))
                .mount(&mock_server)
                .await;
            
            let client = setup_test_client(&mock_server).await;
            
            let upload_request = FileUploadRequest::new(
                b"test content".to_vec(),
                "test.txt",
                "text/plain"
            ).purpose(purpose_str);
            
            let response = client.files().upload(upload_request, None).await;
            assert!(response.is_ok());
            
            let result = response.unwrap();
            assert_eq!(result.file.purpose, purpose_enum);
        }
    }

    #[tokio::test]
    async fn test_upload_file_error_too_large() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/v1/files"))
            .respond_with(ResponseTemplate::new(413)
                .set_body_json(&json!({
                    "type": "error",
                    "error": {
                        "type": "request_too_large",
                        "message": "File size exceeds maximum allowed size"
                    }
                })))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        // Simulate large file
        let large_content = vec![0u8; 10 * 1024 * 1024]; // 10MB
        let upload_request = FileUploadRequest::new(
            large_content,
            "large_file.bin",
            "application/octet-stream"
        ).purpose("user_data");
        
        let response = client.files().upload(upload_request, None).await;
        assert!(response.is_err());
        
        if let Err(threatflux::error::AnthropicError::Api { status, .. }) = response {
            assert_eq!(status, 413);
        } else {
            panic!("Expected 413 error");
        }
    }

    #[tokio::test]
    async fn test_upload_unsupported_file_type() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/v1/files"))
            .respond_with(ResponseTemplate::new(400)
                .set_body_json(&json!({
                    "type": "error",
                    "error": {
                        "type": "invalid_request_error",
                        "message": "Unsupported file type"
                    }
                })))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let upload_request = FileUploadRequest::new(
            b"executable content".to_vec(),
            "malware.exe",
            "application/x-executable"
        ).purpose("user_data");
        
        let response = client.files().upload(upload_request, None).await;
        assert!(response.is_err());
    }

    #[tokio::test]
    async fn test_file_not_found() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/v1/files/nonexistent"))
            .respond_with(ResponseTemplate::new(404)
                .set_body_json(&json!({
                    "type": "error",
                    "error": {
                        "type": "not_found_error",
                        "message": "File not found"
                    }
                })))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let response = client.files().get("nonexistent", None).await;
        assert!(response.is_err());
        
        if let Err(threatflux::error::AnthropicError::Api { status, .. }) = response {
            assert_eq!(status, 404);
        } else {
            panic!("Expected 404 error");
        }
    }

    #[tokio::test]
    async fn test_file_expiration() {
        let mock_server = MockServer::start().await;
        
        let expired_file = json!({
            "id": "file_expired",
            "type": "file",
            "filename": "expired.txt",
            "size_bytes": 100,
            "mime_type": "text/plain",
            "purpose": "user_data",
            "status": "uploaded",
            "created_at": "2024-01-01T00:00:00Z",
            "expires_at": "2024-01-01T01:00:00Z"
        });
        
        Mock::given(method("GET"))
            .and(path("/v1/files/file_expired"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&expired_file))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let response = client.files().get("file_expired", None).await;
        assert!(response.is_ok());
        
        let file = response.unwrap();
        assert!(file.expires_at.is_some());
        
        // Check if file has expired (in this test, it would have expired)
        let now = chrono::Utc::now();
        if let Some(expires_at) = file.expires_at {
            // This would be true if the test runs after the expiry time
            let _is_expired = now > expires_at;
        }
    }

    #[tokio::test]
    async fn test_download_binary_file() {
        let mock_server = MockServer::start().await;
        
        // Simulate binary file content
        let binary_content = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]; // PNG header
        
        Mock::given(method("GET"))
            .and(path("/v1/files/binary_file/content"))
            .respond_with(ResponseTemplate::new(200)
                .set_header("content-type", "image/png")
                .set_header("content-disposition", "attachment; filename=\"image.png\"")
                .set_body_bytes(&binary_content))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let response = client.files().download("binary_file", None).await;
        
        assert!(response.is_ok());
        let download = response.unwrap();
        assert_eq!(download.content, binary_content);
        assert_eq!(download.mime_type, "image/png");
    }

    #[tokio::test]
    async fn test_file_with_unicode_filename() {
        let mock_server = MockServer::start().await;
        
        let unicode_file = json!({
            "file": {
                "id": "file_unicode",
                "type": "file",
                "filename": "测试文件.txt",
                "size_bytes": 20,
                "mime_type": "text/plain",
                "purpose": "user_data",
                "status": "uploaded",
                "created_at": "2024-01-01T00:00:00Z",
                "expires_at": null
            }
        });
        
        Mock::given(method("POST"))
            .and(path("/v1/files"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&unicode_file))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let upload_request = FileUploadRequest::new(
            "测试内容".as_bytes().to_vec(),
            "测试文件.txt",
            "text/plain"
        ).purpose("user_data");
        
        let response = client.files().upload(upload_request, None).await;
        assert!(response.is_ok());
        
        let result = response.unwrap();
        assert_eq!(result.file.filename, "测试文件.txt");
    }

    #[tokio::test]
    async fn test_concurrent_file_operations() {
        let mock_server = MockServer::start().await;
        
        // Set up mocks for multiple file operations
        for i in 0..3 {
            let mut file = fixtures::test_file();
            file.id = format!("file_concurrent_{}", i);
            file.filename = format!("concurrent_{}.txt", i);
            
            let upload_response = threatflux::models::file::FileUploadResponse {
                file: file.clone(),
            };
            
            Mock::given(method("POST"))
                .and(path("/v1/files"))
                .respond_with(ResponseTemplate::new(200)
                    .set_body_json(&upload_response))
                .mount(&mock_server)
                .await;
        }
        
        let client = setup_test_client(&mock_server).await;
        
        // Perform concurrent uploads
        let mut handles = vec![];
        
        for i in 0..3 {
            let client = client.clone();
            let handle = tokio::spawn(async move {
                let upload_request = FileUploadRequest::new(
                    format!("Content {}", i).as_bytes().to_vec(),
                    &format!("concurrent_{}.txt", i),
                    "text/plain"
                ).purpose("user_data");
                
                client.files().upload(upload_request, None).await
            });
            handles.push(handle);
        }
        
        // Wait for all uploads to complete
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }
    }
}