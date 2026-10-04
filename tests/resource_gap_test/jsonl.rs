//! Incremental batch JSONL and transport cancellation.

use super::*;

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
