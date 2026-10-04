use super::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::oneshot,
};

type CallbackSignal = Arc<std::sync::Mutex<Option<oneshot::Sender<()>>>>;

pub(super) struct ControlledDelivery {
    pub(super) address: std::net::SocketAddr,
    pub(super) prefix_ready: oneshot::Receiver<()>,
    pub(super) release: oneshot::Sender<()>,
    pub(super) callback_signal: CallbackSignal,
    pub(super) server: tokio::task::JoinHandle<()>,
}

pub(super) async fn controlled_delivery() -> ControlledDelivery {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (prefix_sent, prefix_ready) = oneshot::channel();
    let (release, terminal_allowed) = oneshot::channel();
    let (callback_fired, callback_observed) = oneshot::channel();
    let callback_signal = Arc::new(std::sync::Mutex::new(Some(callback_fired)));
    let server = tokio::spawn(serve(
        listener,
        prefix_sent,
        terminal_allowed,
        callback_observed,
    ));
    ControlledDelivery {
        address,
        prefix_ready,
        release,
        callback_signal,
        server,
    }
}

pub(super) fn signalling_registry(
    callback_signal: CallbackSignal,
    count: Arc<AtomicUsize>,
) -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    registry
        .register_typed::<Input, _, _>(tool("calculate"), move |input| {
            let signal = callback_signal.clone();
            let count = count.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                signal.lock().unwrap().take().unwrap().send(()).unwrap();
                Ok(ToolResultContent::Text(input.value.to_string()))
            }
        })
        .unwrap();
    registry
}

async fn serve(
    listener: TcpListener,
    prefix_sent: oneshot::Sender<()>,
    terminal_allowed: oneshot::Receiver<()>,
    callback_observed: oneshot::Receiver<()>,
) {
    let (mut socket, _) = listener.accept().await.unwrap();
    read_request(&mut socket).await;
    headers(&mut socket).await;
    chunk(&mut socket, &tool_turn_prefix()).await;
    prefix_sent.send(()).unwrap();
    terminal_allowed.await.unwrap();
    chunk(
        &mut socket,
        &event("message_stop", json!({"type":"message_stop"})),
    )
    .await;
    // Hold the HTTP body open until callback execution: EOF-based collection deadlocks.
    tokio::time::timeout(Duration::from_secs(5), callback_observed)
        .await
        .unwrap()
        .unwrap();
    drop(socket);
    let (mut socket, _) = listener.accept().await.unwrap();
    read_request(&mut socket).await;
    headers(&mut socket).await;
    chunk(&mut socket, &final_turn()).await;
    // The terminal SSE collector may already have closed the transport.
    let _ = socket.write_all(b"0\r\n\r\n").await;
}

async fn read_request(socket: &mut TcpStream) {
    let mut bytes = Vec::new();
    loop {
        let mut chunk = [0u8; 2048];
        let size = socket.read(&mut chunk).await.unwrap();
        assert_ne!(size, 0);
        bytes.extend_from_slice(&chunk[..size]);
        if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&bytes[..end]);
            let length = headers
                .lines()
                .find_map(|line| {
                    line.split_once(':')
                        .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                        .map(|(_, value)| value.trim().parse::<usize>().unwrap())
                })
                .unwrap_or_default();
            if bytes.len() >= end + 4 + length {
                return;
            }
        }
    }
}

async fn headers(socket: &mut TcpStream) {
    socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").await.unwrap();
}

async fn chunk(socket: &mut TcpStream, body: &str) {
    socket
        .write_all(format!("{:x}\r\n", body.len()).as_bytes())
        .await
        .unwrap();
    socket.write_all(body.as_bytes()).await.unwrap();
    socket.write_all(b"\r\n").await.unwrap();
    socket.flush().await.unwrap();
}

fn event(name: &str, body: Value) -> String {
    format!("event: {name}\ndata: {body}\n\n")
}

fn tool_turn_prefix() -> String {
    event(
        "message_start",
        json!({"type":"message_start","message":response(vec![],None)}),
    ) + &event(
        "content_block_start",
        json!({"type":"content_block_start","index":0,"content_block":tool_call("a","calculate",json!({}))}),
    ) + &event(
        "content_block_delta",
        json!({"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"value\":1}"}}),
    ) + &event(
        "content_block_stop",
        json!({"type":"content_block_stop","index":0}),
    ) + &event(
        "message_delta",
        json!({"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":3}}),
    )
}

fn final_turn() -> String {
    event(
        "message_start",
        json!({"type":"message_start","message":response(vec![],None)}),
    ) + &event(
        "message_delta",
        json!({"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":3}}),
    ) + &event("message_stop", json!({"type":"message_stop"}))
}
