//! Chunk boundaries, UTF-8, frame limits and cancellation regressions.

use super::*;

#[tokio::test]
async fn all_chunk_splits_utf8_crlf_multiline_and_final_frame_are_supported() {
    let events = [
        start(),
        block(0, json!({"type":"text","text":""})),
        delta(0, json!({"type":"text_delta","text":"é😀"})),
        stop_block(0),
        stop(),
    ];
    let mut bytes = String::from_utf8(events.iter().flat_map(frame).collect())
        .unwrap()
        .replace('\n', "\r\n")
        .into_bytes();
    // The terminal JSON is complete, but no final line or blank-line delimiter.
    bytes.truncate(bytes.len() - 4);
    for split in 0..=bytes.len() {
        let chunks = vec![Ok(bytes[..split].to_vec()), Ok(bytes[split..].to_vec())];
        let response = MessageStream::from_bytes(stream::iter(chunks), StreamLimits::default())
            .collect_message()
            .await
            .unwrap();
        assert_eq!(response.text(), "é😀", "split at {split}");
    }
    let chunks: Vec<reqwest::Result<Vec<u8>>> = bytes.iter().map(|byte| Ok(vec![*byte])).collect();
    assert_eq!(
        MessageStream::from_bytes(stream::iter(chunks), StreamLimits::default())
            .collect_text()
            .await
            .unwrap(),
        "é😀"
    );
    let multiline=b"event: message_start\ndata: {\ndata: \"type\":\"message_start\",\ndata: \"message\":{\"id\":\"m\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"custom\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"usage\":{}}\ndata: }\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}";
    assert!(MessageStream::from_bytes(
        stream::iter(vec![Ok(multiline.to_vec())]),
        StreamLimits::default()
    )
    .collect_message()
    .await
    .is_ok());
}

#[tokio::test]
async fn invalid_utf8_oversized_lines_and_tool_input_are_bounded() {
    let invalid = vec![Ok(b"data: \xff\n\n".to_vec())];
    let mut raw = MessageStream::from_bytes(stream::iter(invalid), StreamLimits::default());
    assert!(raw.next().await.unwrap().is_err());
    assert!(raw.next().await.is_none());
    let limits = StreamLimits {
        max_event_bytes: 8,
        ..StreamLimits::default()
    };
    let mut raw =
        MessageStream::from_bytes(stream::iter(vec![Ok(b"data: 123456789".to_vec())]), limits);
    assert!(raw.next().await.unwrap().is_err());
    let mut acc = MessageAccumulator::new(StreamLimits {
        max_tool_input_bytes: 2,
        ..StreamLimits::default()
    });
    let parser = EventParser::new();
    for event in [
        start(),
        block(
            0,
            json!({"type":"tool_use","id":"t","name":"test","input":{}}),
        ),
    ] {
        acc.apply(
            parser
                .parse_event(event["type"].as_str().unwrap(), &event.to_string())
                .unwrap(),
        )
        .unwrap();
    }
    assert!(acc
        .apply(
            parser
                .parse_event(
                    "content_block_delta",
                    &delta(
                        0,
                        json!({"type":"input_json_delta","partial_json":"{\"large\":1}"})
                    )
                    .to_string()
                )
                .unwrap()
        )
        .is_err());
}

#[tokio::test]
async fn events_arrive_incrementally_and_unknown_events_remain_raw() {
    let (sender, receiver) = mpsc::channel::<reqwest::Result<Vec<u8>>>(2);
    let mut stream = MessageStream::from_bytes(
        tokio_stream::wrappers::ReceiverStream::new(receiver),
        StreamLimits::default(),
    );
    sender.send(Ok(frame(&start()))).await.unwrap();
    assert!(matches!(
        timeout(Duration::from_secs(1), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
        StreamEvent::MessageStart { .. }
    ));
    sender
        .send(Ok(b"event: future_event\ndata: opaque text\n\n".to_vec()))
        .await
        .unwrap();
    assert!(
        matches!(stream.next().await.unwrap().unwrap(),StreamEvent::Unknown{event_type,data} if event_type=="future_event" && data=="opaque text")
    );
    drop(sender);
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn terminal_collection_cancels_a_body_that_never_reaches_http_eof() {
    for text_only in [false, true] {
        let (sender, receiver) = mpsc::channel::<reqwest::Result<Vec<u8>>>(1);
        let stream = MessageStream::from_bytes(
            tokio_stream::wrappers::ReceiverStream::new(receiver),
            StreamLimits::default(),
        );
        let body = [
            start(),
            block(0, json!({"type":"text","text":""})),
            delta(0, json!({"type":"text_delta","text":"completed"})),
            stop_block(0),
            stop(),
        ]
        .iter()
        .flat_map(frame)
        .collect();
        sender.send(Ok(body)).await.unwrap();
        // Keeping the sender alive deliberately withholds HTTP EOF.
        let collected = timeout(Duration::from_secs(1), async move {
            if text_only {
                stream.collect_text().await
            } else {
                stream.collect_message().await.map(|message| {
                    message
                        .content
                        .iter()
                        .filter_map(ContentBlock::as_text)
                        .collect()
                })
            }
        })
        .await
        .expect("message_stop must complete collection without HTTP EOF")
        .unwrap();
        assert_eq!(collected, "completed");
        timeout(Duration::from_secs(1), sender.closed())
            .await
            .expect("collection must cancel the body producer");
    }
}

#[tokio::test]
async fn same_chunk_terminal_duplicates_fail_before_terminal_publication() {
    let body = [start(), stop(), stop()]
        .iter()
        .flat_map(frame)
        .collect::<Vec<_>>();
    let error = MessageStream::from_bytes(stream::iter(vec![Ok(body)]), StreamLimits::default())
        .collect_message()
        .await
        .unwrap_err();
    assert!(matches!(error, AnthropicError::Stream(_)));
}

#[tokio::test]
async fn raw_events_keep_the_http_body_alive_after_message_stop() {
    let (sender, receiver) = mpsc::channel::<reqwest::Result<Vec<u8>>>(1);
    let mut stream = MessageStream::from_bytes(
        tokio_stream::wrappers::ReceiverStream::new(receiver),
        StreamLimits::default(),
    );
    sender.send(Ok(frame(&stop()))).await.unwrap();
    assert!(matches!(
        stream.next().await.unwrap().unwrap(),
        StreamEvent::MessageStop
    ));
    assert!(timeout(Duration::from_millis(20), stream.next())
        .await
        .is_err());
    sender
        .send(Ok(b"event: future_event\ndata: retained\n\n".to_vec()))
        .await
        .unwrap();
    assert!(matches!(
        stream.next().await.unwrap().unwrap(),
        StreamEvent::Unknown { .. }
    ));
    stream.close();
}

struct PendingBytes {
    dropped: Option<oneshot::Sender<()>>,
}
impl Stream for PendingBytes {
    type Item = reqwest::Result<Vec<u8>>;
    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Pending
    }
}
impl Drop for PendingBytes {
    fn drop(&mut self) {
        if let Some(sender) = self.dropped.take() {
            let _ = sender.send(());
        }
    }
}
#[tokio::test]
async fn dropping_consumer_cancels_even_a_pending_http_body() {
    let (sender, receiver) = oneshot::channel();
    let stream = MessageStream::from_bytes(
        PendingBytes {
            dropped: Some(sender),
        },
        StreamLimits::default(),
    );
    tokio::task::yield_now().await;
    drop(stream);
    timeout(Duration::from_secs(1), receiver)
        .await
        .unwrap()
        .unwrap();
}
