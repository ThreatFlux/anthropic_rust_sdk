//! Legacy text completions API implementation.

use crate::{
    client::Client,
    error::Result,
    models::completion::{CompletionRequest, CompletionResponse},
    types::{HttpMethod, RequestOptions},
};

/// API client for legacy `/v1/complete` endpoint.
#[derive(Clone)]
pub struct CompletionsApi {
    client: Client,
}

impl CompletionsApi {
    /// Create a new Completions API client.
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Create a legacy text completion.
    pub async fn create(
        &self,
        request: CompletionRequest,
        options: Option<RequestOptions>,
    ) -> Result<CompletionResponse> {
        let body = serde_json::to_value(request)?;
        self.client
            .request(HttpMethod::Post, "/complete", Some(body), options)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;
    use serde_json::json;
    use wiremock::{
        matchers::{body_partial_json, method},
        Mock, MockServer, ResponseTemplate,
    };

    #[tokio::test]
    async fn test_create_completion() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(body_partial_json(json!({
                "model": "claude-2.1",
                "prompt": "\n\nHuman: Hello\n\nAssistant:",
                "max_tokens_to_sample": 64
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "compl_123",
                "type": "completion",
                "completion": "Hello!",
                "model": "claude-2.1",
                "stop_reason": "stop_sequence",
                "stop": "\n\nHuman:"
            })))
            .mount(&server)
            .await;

        let config = Config::new("test-key")
            .unwrap()
            .with_base_url(server.uri().parse().unwrap());
        let client = Client::new(config);
        let api = CompletionsApi::new(client);

        let response = api
            .create(
                CompletionRequest::new("\n\nHuman: Hello\n\nAssistant:", 64).model("claude-2.1"),
                None,
            )
            .await
            .unwrap();

        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests[0].url.path(), "/v1/complete");
        assert_eq!(response.object_type, "completion");
        assert_eq!(response.completion, "Hello!");
    }
}
