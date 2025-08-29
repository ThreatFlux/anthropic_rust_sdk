//! Shared utilities for API modules

use crate::types::Pagination;

/// Builds query parameters for pagination
pub fn build_pagination_query(pagination: &Pagination) -> Vec<String> {
    let mut query_params = Vec::new();

    if let Some(limit) = pagination.limit {
        query_params.push(format!("limit={}", limit));
    }

    if let Some(after) = &pagination.after {
        query_params.push(format!("after={}", after));
    }

    if let Some(before) = &pagination.before {
        query_params.push(format!("before={}", before));
    }

    query_params
}

/// Builds a path with query parameters
pub fn build_path_with_query(base_path: &str, query_params: Vec<String>) -> String {
    let mut path = base_path.to_string();

    if !query_params.is_empty() {
        path.push('?');
        path.push_str(&query_params.join("&"));
    }

    path
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
    Pagination::new()
        .with_limit(100)
        .with_after(after.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(pagination.after, Some(String::new()));
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
