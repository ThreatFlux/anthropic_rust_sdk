//! Shared utilities for API modules

use crate::{
    error::{AnthropicError, Result},
    types::{PageStream, Pagination, PaginationLimits},
};
use std::collections::HashSet;

/// An endpoint page normalized for bounded traversal.
pub(crate) struct TraversalPage<T> {
    pub data: Vec<T>,
    pub next_cursor: Option<String>,
    pub item_ids: Vec<String>,
}

/// Construct a lazy, bounded traversal shared by ID- and token-cursor APIs.
pub(crate) fn paginate<T, F, Fut>(
    limits: PaginationLimits,
    initial_cursor: Option<String>,
    fetch: F,
) -> Result<PageStream<T>>
where
    T: Send + 'static,
    F: FnMut(Option<String>) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<TraversalPage<T>>> + Send,
{
    limits.validate()?;
    if initial_cursor.as_deref() == Some("") {
        return Err(AnthropicError::invalid_input(
            "Pagination cursor must not be empty",
        ));
    }
    let mut seen_cursors = HashSet::new();
    if let Some(cursor) = &initial_cursor {
        seen_cursors.insert(cursor.clone());
    }
    let state = (
        initial_cursor,
        seen_cursors,
        HashSet::<String>::new(),
        0_usize,
        0_usize,
        false,
        fetch,
    );
    Ok(PageStream::new(futures::stream::try_unfold(
        state,
        move |state| async move {
            let (cursor, mut seen_cursors, mut seen_items, pages, items, done, mut fetch) = state;
            if done {
                return Ok(None);
            }
            if pages >= limits.max_pages || items >= limits.max_items {
                return Err(AnthropicError::invalid_input(
                    "Pagination traversal limit reached while more pages remain",
                ));
            }
            let page = fetch(cursor).await?;
            if page.next_cursor.as_deref() == Some("") {
                return Err(AnthropicError::invalid_input(
                    "Pagination response contains an empty next cursor",
                ));
            }
            if page.next_cursor.is_some() && page.data.is_empty() {
                return Err(AnthropicError::invalid_input(
                    "Pagination returned an empty continuing page",
                ));
            }
            if let Some(next) = &page.next_cursor {
                if !seen_cursors.insert(next.clone()) {
                    return Err(AnthropicError::invalid_input(
                        "Pagination cursor repeated or formed a cycle",
                    ));
                }
            }
            let new_items = page
                .item_ids
                .iter()
                .filter(|id| seen_items.insert((*id).clone()))
                .count();
            if !page.data.is_empty() && !page.item_ids.is_empty() && new_items == 0 {
                return Err(AnthropicError::invalid_input(
                    "Pagination returned no new items",
                ));
            }
            let count = items
                .checked_add(page.data.len())
                .ok_or_else(|| AnthropicError::invalid_input("Pagination item count overflow"))?;
            if count > limits.max_items {
                return Err(AnthropicError::invalid_input(
                    "Pagination item limit exceeded",
                ));
            }
            let done = page.next_cursor.is_none();
            Ok(Some((
                page.data,
                (
                    page.next_cursor,
                    seen_cursors,
                    seen_items,
                    pages + 1,
                    count,
                    done,
                    fetch,
                ),
            )))
        },
    )))
}

/// Require a continuation cursor for ID-based pages advertising more data.
pub(crate) fn id_cursor(has_more: bool, cursor: Option<String>) -> Result<Option<String>> {
    if has_more {
        match cursor {
            Some(cursor) if !cursor.is_empty() => Ok(Some(cursor)),
            _ => Err(AnthropicError::invalid_input(
                "Pagination response has_more without a usable cursor",
            )),
        }
    } else {
        Ok(None)
    }
}

/// URL-encode one query value using the same encoding as structured query builders.
pub fn encode_query_value(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

/// Builds query parameters for pagination
pub fn build_pagination_query(pagination: &Pagination) -> Vec<String> {
    let mut query_params = Vec::new();

    if let Some(limit) = pagination.limit {
        query_params.push(format!("limit={}", limit));
    }

    if let Some(after) = &pagination.after {
        query_params.push(format!("after={}", encode_query_value(after)));
    }

    if let Some(before) = &pagination.before {
        query_params.push(format!("before={}", encode_query_value(before)));
    }

    query_params
}

/// Builds a path with query parameters
pub fn build_path_with_query(base_path: &str, query_params: Vec<String>) -> String {
    let mut path = base_path.to_string();

    if !query_params.is_empty() {
        path.push(if base_path.contains('?') { '&' } else { '?' });
        path.push_str(&query_params.join("&"));
    }

    path
}

/// Builds a path with URL-encoded key/value query parameters.
pub fn build_query_path(base_path: &str, query_params: Vec<(String, String)>) -> String {
    if query_params.is_empty() {
        return base_path.to_string();
    }

    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    for (key, value) in query_params {
        serializer.append_pair(&key, &value);
    }
    let separator = if base_path.contains('?') { '&' } else { '?' };
    format!("{}{}{}", base_path, separator, serializer.finish())
}

/// Builds pagination query parameters and adds them to a path
pub fn build_paginated_path(base_path: &str, pagination: Option<&Pagination>) -> String {
    if let Some(pagination) = pagination {
        let query_params = build_pagination_query(pagination);
        build_path_with_query(base_path, query_params)
    } else {
        base_path.to_string()
    }
}

/// Creates a default pagination for list_all operations
pub fn create_default_pagination(after: Option<String>) -> Pagination {
    Pagination {
        limit: Some(100),
        after,
        before: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn traversal_rejects_cursor_cycles_empty_pages_and_unchanged_items() {
        for scenario in 0..3 {
            let mut requests = 0;
            let mut stream = paginate(PaginationLimits::default(), None, move |_| {
                requests += 1;
                let page = if requests == 1 {
                    TraversalPage {
                        data: vec![1],
                        next_cursor: Some("first".into()),
                        item_ids: vec!["one".into()],
                    }
                } else {
                    match scenario {
                        0 => TraversalPage {
                            data: vec![2],
                            next_cursor: Some("first".into()),
                            item_ids: vec!["two".into()],
                        },
                        1 => TraversalPage {
                            data: Vec::new(),
                            next_cursor: Some("second".into()),
                            item_ids: Vec::new(),
                        },
                        _ => TraversalPage {
                            data: vec![1],
                            next_cursor: Some("second".into()),
                            item_ids: vec!["one".into()],
                        },
                    }
                };
                async move { Ok(page) }
            })
            .unwrap();
            assert_eq!(stream.next().await.unwrap().unwrap(), vec![1]);
            assert!(stream.next().await.unwrap().is_err());
            assert!(stream.next().await.is_none());
        }
    }

    #[tokio::test]
    async fn traversal_limits_do_not_fetch_an_extra_page_or_return_partial_collection() {
        for limits in [
            PaginationLimits::new(1, 20).unwrap(),
            PaginationLimits::new(20, 1).unwrap(),
        ] {
            let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let observed = count.clone();
            let stream = paginate(limits, None, move |_| {
                observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                async {
                    Ok(TraversalPage {
                        data: vec![1],
                        next_cursor: Some("next".into()),
                        item_ids: vec!["one".into()],
                    })
                }
            })
            .unwrap();
            assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 0);
            assert!(stream.collect_items().await.is_err());
            assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 1);
        }
    }

    #[tokio::test]
    async fn traversal_three_pages_and_empty_terminal_page_are_valid() {
        let mut counter = 0;
        let stream = paginate(PaginationLimits::new(3, 3).unwrap(), None, move |_| {
            counter += 1;
            let page = if counter < 3 {
                TraversalPage {
                    data: vec![counter],
                    next_cursor: Some(counter.to_string()),
                    item_ids: vec![counter.to_string()],
                }
            } else {
                TraversalPage {
                    data: Vec::new(),
                    next_cursor: None,
                    item_ids: Vec::new(),
                }
            };
            async move { Ok(page) }
        })
        .unwrap();
        assert_eq!(stream.collect_items().await.unwrap(), vec![1, 2]);
        let empty = paginate(PaginationLimits::default(), None, |_| async {
            Ok(TraversalPage::<u32> {
                data: Vec::new(),
                next_cursor: None,
                item_ids: Vec::new(),
            })
        })
        .unwrap();
        assert!(empty.collect_items().await.unwrap().is_empty());
    }

    #[test]
    fn id_cursor_contract_and_encoding() {
        assert!(id_cursor(true, None).is_err());
        assert!(id_cursor(true, Some(String::new())).is_err());
        assert_eq!(id_cursor(false, Some("ignored".into())).unwrap(), None);
        assert_eq!(
            build_paginated_path(
                "/models",
                Some(&Pagination::new().with_after("x&before=bad"))
            ),
            "/models?limit=20&after=x%26before%3Dbad"
        );
        assert!(Pagination::new()
            .with_after("a")
            .with_before("b")
            .validate()
            .is_err());
    }

    #[test]
    fn test_build_pagination_query_empty() {
        let pagination = Pagination {
            limit: None,
            after: None,
            before: None,
        };
        let query = build_pagination_query(&pagination);
        assert!(query.is_empty());
    }

    #[test]
    fn test_build_pagination_query_with_limit() {
        let pagination = Pagination::new().with_limit(50);
        let query = build_pagination_query(&pagination);
        assert_eq!(query, vec!["limit=50"]);
    }

    #[test]
    fn test_build_pagination_query_full() {
        let pagination = Pagination::new()
            .with_limit(50)
            .with_after("after_id".to_string())
            .with_before("before_id".to_string());
        let query = build_pagination_query(&pagination);
        assert_eq!(
            query,
            vec!["limit=50", "after=after_id", "before=before_id"]
        );
    }

    #[test]
    fn test_build_path_with_query_empty() {
        let path = build_path_with_query("/test", vec![]);
        assert_eq!(path, "/test");
    }

    #[test]
    fn test_build_path_with_query_params() {
        let path = build_path_with_query(
            "/test",
            vec!["limit=50".to_string(), "after=123".to_string()],
        );
        assert_eq!(path, "/test?limit=50&after=123");
    }

    #[test]
    fn test_build_query_path_encodes_values() {
        let path = build_query_path(
            "/dreams",
            vec![(
                "created_at[gt]".to_string(),
                "2026-07-01T00:00:00Z".to_string(),
            )],
        );
        assert_eq!(path, "/dreams?created_at%5Bgt%5D=2026-07-01T00%3A00%3A00Z");
    }

    #[test]
    fn test_build_query_path_appends_to_existing_query() {
        let path = build_query_path(
            "/dreams?beta=true",
            vec![("limit".to_string(), "10".to_string())],
        );
        assert_eq!(path, "/dreams?beta=true&limit=10");
    }

    #[test]
    fn test_build_paginated_path_none() {
        let path = build_paginated_path("/test", None);
        assert_eq!(path, "/test");
    }

    #[test]
    fn test_build_paginated_path_some() {
        let pagination = Pagination::new().with_limit(25);
        let path = build_paginated_path("/test", Some(&pagination));
        assert_eq!(path, "/test?limit=25");
    }

    #[test]
    fn test_create_default_pagination_no_after() {
        let pagination = create_default_pagination(None);
        assert_eq!(pagination.limit, Some(100));
        assert_eq!(pagination.after, None);
        assert_eq!(pagination.before, None);
    }

    #[test]
    fn test_create_default_pagination_with_after() {
        let pagination = create_default_pagination(Some("test_id".to_string()));
        assert_eq!(pagination.limit, Some(100));
        assert_eq!(pagination.after, Some("test_id".to_string()));
        assert_eq!(pagination.before, None);
    }
}
