//! Lifecycle ordering and truncation regressions.

use super::*;

#[tokio::test]
async fn every_lifecycle_truncation_fails_for_both_collectors() {
    let events = [
        start(),
        block(0, json!({"type":"text","text":""})),
        delta(0, json!({"type":"text_delta","text":"hello"})),
        stop_block(0),
        json!({"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":4}}),
        stop(),
    ];
    for truncated in 0..events.len() {
        assert!(
            fixture_stream(&events[..truncated])
                .collect_message()
                .await
                .is_err(),
            "truncated at {truncated}"
        );
        assert!(
            fixture_stream(&events[..truncated])
                .collect_text()
                .await
                .is_err(),
            "text truncated at {truncated}"
        );
    }
    assert_eq!(
        fixture_stream(&events).collect_text().await.unwrap(),
        "hello"
    );
    assert!(fixture_stream(&[start(), stop()])
        .collect_text()
        .await
        .unwrap()
        .is_empty());
}

#[test]
fn invalid_order_indices_duplicate_events_and_open_blocks_fail() {
    for events in [
        vec![stop()],
        vec![start(), start(), stop()],
        vec![start(), stop(), stop()],
        vec![start(), block(1, json!({"type":"text","text":""})), stop()],
        vec![
            start(),
            delta(0, json!({"type":"text_delta","text":"x"})),
            stop(),
        ],
        vec![start(), block(0, json!({"type":"text","text":""})), stop()],
        vec![
            start(),
            block(0, json!({"type":"text","text":""})),
            stop_block(0),
            stop_block(0),
            stop(),
        ],
        vec![
            start(),
            block(0, json!({"type":"text","text":""})),
            stop_block(0),
            delta(0, json!({"type":"text_delta","text":"x"})),
            stop(),
        ],
    ] {
        assert!(accumulator(&events).is_err(), "invalid fixture {events:?}");
    }
}
