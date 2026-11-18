//! Actual public model, calendar-date and HTTP-header timestamp contracts.

use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::json;
use std::time::Duration;
use threatflux_anthropic_sdk::{
    models::{admin::ClaudeCodeUsageReportParams, file::File},
    utils::http::HttpClient,
    Date, Timestamp,
};

fn file(created: &str) -> serde_json::Value {
    json!({"id": "file_fixture", "type": "file", "filename": "fixture.txt",
        "mime_type": "text/plain", "size_bytes": 3, "created_at": created,
        "expires_at": null, "updated_at": null, "provider_extra": {"future": 1}})
}

#[test]
fn file_model_preserves_offset_nanoseconds_optional_values_and_unknown_metadata() {
    let record: File = serde_json::from_value(file("2026-11-18T02:00:00.123456789+02:00")).unwrap();
    let expected: Timestamp = "2026-11-18T00:00:00.123456789Z".parse().unwrap();
    assert_eq!(record.created_at, expected);
    assert!(record.expires_at.is_none());
    let output = serde_json::to_value(record).unwrap();
    assert_eq!(
        output["created_at"],
        json!("2026-11-18T00:00:00.123456789Z")
    );
    assert_eq!(output["updated_at"], serde_json::Value::Null);
    assert_eq!(output["provider_extra"], json!({"future": 1}));
}

#[test]
fn invalid_file_dates_fail_and_leap_seconds_are_explicitly_normalized() {
    for invalid in ["2026-02-30T00:00:00Z", "2026-11-18T00:00:00", "not-a-date"] {
        assert!(serde_json::from_value::<File>(file(invalid)).is_err());
    }
    let record: File = serde_json::from_value(file("2016-12-31T23:59:60.5Z")).unwrap();
    assert_eq!(
        serde_json::to_value(record).unwrap()["created_at"],
        json!("2016-12-31T23:59:59.5Z")
    );
}

#[test]
fn claude_usage_calendar_dates_remain_date_only() {
    let params = ClaudeCodeUsageReportParams::new(Date::new(2026, 1, 1).unwrap())
        .ending_at(Date::new(2026, 1, 2).unwrap());
    let wire = serde_json::to_value(params).unwrap();
    assert_eq!(wire["starting_at"], json!("2026-01-01"));
    assert_eq!(wire["ending_at"], json!("2026-01-02"));
    assert!(Date::new(2026, 2, 30).is_err());
}

#[test]
fn reset_header_bounds_do_not_panic_or_invent_current_time_and_backoff_stays_capped() {
    let mut headers = HeaderMap::new();
    headers.insert("x-ratelimit-limit", HeaderValue::from_static("100"));
    headers.insert("x-ratelimit-remaining", HeaderValue::from_static("0"));
    for invalid in [i64::MAX, i64::MIN] {
        headers.insert("x-ratelimit-reset", invalid.to_string().parse().unwrap());
        assert!(HttpClient::parse_rate_limit_headers(&headers)
            .reset
            .is_none());
    }
    headers.insert(
        "x-ratelimit-reset",
        Timestamp::MAX.as_second().to_string().parse().unwrap(),
    );
    let info = HttpClient::parse_rate_limit_headers(&headers);
    assert_eq!(info.reset, Some(Timestamp::MAX));
    assert_eq!(info.recommended_delay(), Some(Duration::from_secs(60)));
    headers.insert("retry-after", HeaderValue::from_static("123"));
    assert_eq!(
        HttpClient::parse_rate_limit_headers(&headers).recommended_delay(),
        Some(Duration::from_secs(123))
    );
}
