//! Message streaming through HTTP, including actual chunked incremental delivery.

use futures::StreamExt;
use serde_json::{json, Value};
use threatflux_anthropic_sdk::{
    models::{message::StreamEvent, MessageRequest},
    Client, Config,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
    time::{timeout, Duration},
};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

fn response() -> Value {
    json!({"id":"msg_wire","type":"message","role":"assistant","model":"future-model","content":[],"stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":15,"output_tokens":0},"diagnostics":{"trace":"retained"},"future":"kept"})
}
fn frame(event: &Value) -> Vec<u8> {
    format!(
        "event: {}\ndata: {}\n\n",
        event["type"].as_str().unwrap(),
        event
    )
    .into_bytes()
}
fn client(uri: &str) -> Client {
    Client::new(
        Config::new("sk-ant-test-key")
            .unwrap()
            .with_base_url(uri.parse().unwrap()),
    )
}

fn equivalent_payload_fixture() -> (Value, Vec<u8>) {
    let mut expected = response();
    expected["content"] = json!([{"type":"text","text":"é😀","future":{"retained":true}}]);
    expected["stop_reason"] = json!("future_stop_reason");
    expected["usage"]["output_tokens"] = json!(3);
    let mut bytes = Vec::new();
    for event in [
        json!({"type":"message_start","message":response()}),
        json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":"","future":{"retained":true}}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"é😀"}}),
        json!({"type":"content_block_stop","index":0}),
        json!({"type":"message_delta","delta":{"stop_reason":"future_stop_reason"},"usage":{"output_tokens":3}}),
        json!({"type":"message_stop"}),
    ] {
        bytes.extend(frame(&event));
    }
    (expected, bytes)
}

#[tokio::test]
async fn collected_http_stream_matches_equivalent_message_payload() {
    let server = MockServer::start().await;
    let (expected, bytes) = equivalent_payload_fixture();
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(bytes)
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let actual = client(&server.uri())
        .messages()
        .create_stream(
            MessageRequest::new()
                .model("future-model")
                .add_user_message("hi"),
            None,
        )
        .await
        .unwrap()
        .collect_message()
        .await
        .unwrap();
    let actual = serde_json::to_value(actual).unwrap();
    for key in [
        "id",
        "model",
        "content",
        "stop_reason",
        "diagnostics",
        "future",
    ] {
        assert_eq!(actual[key], expected[key], "{key}");
    }
    assert_eq!(
        actual["usage"]["output_tokens"],
        expected["usage"]["output_tokens"]
    );
    assert_eq!(
        actual["usage"]["input_tokens"],
        expected["usage"]["input_tokens"]
    );
}

#[tokio::test]
async fn http_eof_during_open_content_is_an_error_for_text_collection() {
    let server = MockServer::start().await;
    let mut bytes = frame(&json!({"type":"message_start","message":response()}));
    bytes.extend(frame(
        &json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}),
    ));
    bytes.extend(frame(&json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"partial"}})));
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(bytes)
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;
    let result = client(&server.uri())
        .messages()
        .create_stream(
            MessageRequest::new()
                .model("future-model")
                .add_user_message("hi"),
            None,
        )
        .await
        .unwrap()
        .collect_text()
        .await;
    assert!(matches!(
        result,
        Err(threatflux_anthropic_sdk::error::AnthropicError::Stream(_))
    ));
}

async fn write_chunk(socket: &mut tokio::net::TcpStream, chunk: &[u8]) {
    socket
        .write_all(format!("{:x}\r\n", chunk.len()).as_bytes())
        .await
        .unwrap();
    socket.write_all(chunk).await.unwrap();
    socket.write_all(b"\r\n").await.unwrap();
    socket.flush().await.unwrap();
}

async fn accept_sse(listener: TcpListener) -> tokio::net::TcpStream {
    let (mut socket, _) = listener.accept().await.unwrap();
    let mut request = [0u8; 8192];
    let _ = socket.read(&mut request).await.unwrap();
    socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").await.unwrap();
    socket
}

async fn serve_split_utf8(listener: TcpListener, continue_receiver: oneshot::Receiver<()>) {
    let mut socket = accept_sse(listener).await;
    write_chunk(
        &mut socket,
        &frame(&json!({"type":"message_start","message":response()})),
    )
    .await;
    continue_receiver.await.unwrap();
    let mut remainder = Vec::new();
    for event in [
        json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"é😀"}}),
        json!({"type":"content_block_stop","index":0}),
        json!({"type":"message_stop"}),
    ] {
        remainder.extend(frame(&event));
    }
    for byte in remainder {
        write_chunk(&mut socket, &[byte]).await;
    }
    socket.write_all(b"0\r\n\r\n").await.unwrap();
}

async fn serve_pending_terminal(listener: TcpListener, release_receiver: oneshot::Receiver<()>) {
    let mut socket = accept_sse(listener).await;
    let mut body = Vec::new();
    for event in [
        json!({"type":"message_start","message":response()}),
        json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"finished"}}),
        json!({"type":"content_block_stop","index":0}),
        json!({"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":2}}),
        json!({"type":"message_stop"}),
    ] {
        body.extend(frame(&event));
    }
    write_chunk(&mut socket, &body).await;
    // This signal is sent only after collection has returned. Withhold
    // the HTTP zero chunk so terminal SSE, rather than HTTP EOF, wins.
    release_receiver.await.unwrap();
}

#[tokio::test]
async fn chunked_http_yields_before_completion_and_keeps_split_utf8() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (continue_sender, continue_receiver) = oneshot::channel();
    let task = tokio::spawn(serve_split_utf8(listener, continue_receiver));
    let mut stream = client(&format!("http://{address}"))
        .messages()
        .create_stream(
            MessageRequest::new()
                .model("future-model")
                .add_user_message("hi"),
            None,
        )
        .await
        .unwrap();
    let first = timeout(Duration::from_secs(2), stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(matches!(first, StreamEvent::MessageStart { .. }));
    // The server is blocked until this signal: receipt of the first event proves
    // the implementation did not buffer the full HTTP body.
    continue_sender.send(()).unwrap();
    let mut text = String::new();
    let mut terminal = false;
    while let Some(event) = stream.next().await {
        match event.unwrap() {
            StreamEvent::ContentBlockDelta { delta, .. } => {
                text.push_str(delta.text.as_deref().unwrap_or_default())
            }
            StreamEvent::MessageStop => terminal = true,
            _ => {}
        }
    }
    assert_eq!(text, "é😀");
    assert!(terminal);
    task.await.unwrap();
}

#[tokio::test]
async fn valid_message_stop_finishes_collectors_while_chunked_http_body_stays_open() {
    for text_only in [false, true] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (release_sender, release_receiver) = oneshot::channel();
        let task = tokio::spawn(serve_pending_terminal(listener, release_receiver));
        let stream = client(&format!("http://{address}"))
            .messages()
            .create_stream(
                MessageRequest::new()
                    .model("future-model")
                    .add_user_message("hi"),
                None,
            )
            .await
            .unwrap();
        let text = timeout(Duration::from_secs(2), async move {
            if text_only {
                stream.collect_text().await
            } else {
                stream.collect_message().await.map(|message| {
                    assert_eq!(message.usage.output_tokens, 2);
                    message
                        .content
                        .iter()
                        .filter_map(|block| block.as_text())
                        .collect::<String>()
                })
            }
        })
        .await
        .expect("complete SSE must not wait for the HTTP zero chunk")
        .unwrap();
        assert_eq!(text, "finished");
        release_sender.send(()).unwrap();
        task.await.unwrap();
    }
}
