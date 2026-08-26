//! Shared pagination for List*/Describe* operations.
//!
//! AWS APIs return at most N items per call and hand back an opaque token for
//! the next page. This module implements that with an offset-based token
//! (the stringified start index), which is sufficient for an in-memory mock.
//!
//! Callers are responsible for producing a *stably ordered* `Vec` (sort by
//! key/name/ARN) before paginating, since DashMap iteration order is random.

use std::collections::HashMap;

use serde_json::Value;

/// One page of results plus the token for the next page (if any).
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_token: Option<String>,
}

impl<T> Page<T> {
    pub fn is_truncated(&self) -> bool {
        self.next_token.is_some()
    }
}

/// Pagination inputs parsed from a request.
#[derive(Debug, Clone, Copy, Default)]
pub struct PageRequest {
    pub start: usize,
    pub limit: Option<usize>,
}

impl PageRequest {
    /// Parse from a JSON body. `limit_keys`/`token_keys` are tried in order
    /// (e.g. `&["maxResults", "MaxResults"]`, `&["nextToken", "NextToken"]`).
    pub fn from_json(payload: &Value, limit_keys: &[&str], token_keys: &[&str]) -> Self {
        let limit = limit_keys.iter().find_map(|k| match &payload[*k] {
            Value::Number(n) => n.as_u64().map(|n| n as usize),
            Value::String(s) => s.parse().ok(),
            _ => None,
        });
        let start = token_keys
            .iter()
            .find_map(|k| payload[*k].as_str())
            .and_then(parse_token)
            .unwrap_or(0);
        Self { start, limit }
    }

    /// Parse from query-string / form params (`HashMap<String, String>`).
    pub fn from_params(
        params: &HashMap<String, String>,
        limit_keys: &[&str],
        token_keys: &[&str],
    ) -> Self {
        let limit = limit_keys
            .iter()
            .find_map(|k| params.get(*k))
            .and_then(|v| v.parse().ok());
        let start = token_keys
            .iter()
            .find_map(|k| params.get(*k))
            .and_then(|s| parse_token(s))
            .unwrap_or(0);
        Self { start, limit }
    }

    /// Slice `items` according to this request, using `default_limit` when
    /// the client did not specify one. A limit of 0 is treated as the default.
    pub fn apply<T>(&self, items: Vec<T>, default_limit: usize) -> Page<T> {
        let limit = match self.limit {
            Some(0) | None => default_limit,
            Some(n) => n,
        };
        let total = items.len();
        let start = self.start.min(total);
        let end = (start + limit).min(total);
        let items: Vec<T> = items.into_iter().skip(start).take(end - start).collect();
        let next_token = if end < total {
            Some(end.to_string())
        } else {
            None
        };
        Page { items, next_token }
    }
}

fn parse_token(s: &str) -> Option<usize> {
    let s = s.trim();
    if s.is_empty() {
        None
    } else {
        s.parse().ok()
    }
}

/// Convenience: paginate a JSON body request in one call.
pub fn paginate_json<T>(
    payload: &Value,
    items: Vec<T>,
    limit_keys: &[&str],
    token_keys: &[&str],
    default_limit: usize,
) -> Page<T> {
    PageRequest::from_json(payload, limit_keys, token_keys).apply(items, default_limit)
}

/// Convenience: paginate a query/form params request in one call.
pub fn paginate_params<T>(
    params: &HashMap<String, String>,
    items: Vec<T>,
    limit_keys: &[&str],
    token_keys: &[&str],
    default_limit: usize,
) -> Page<T> {
    PageRequest::from_params(params, limit_keys, token_keys).apply(items, default_limit)
}

/// Query-protocol (IAM/STS/SNS/etc.) XML tail: `<IsTruncated>` plus `<Marker>`.
pub fn query_xml_tail<T>(page: &Page<T>) -> String {
    match &page.next_token {
        Some(t) => format!("<IsTruncated>true</IsTruncated><Marker>{t}</Marker>"),
        None => "<IsTruncated>false</IsTruncated>".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn default_limit_and_token() {
        let items: Vec<u32> = (0..197).collect();
        let p = paginate_json(&json!({}), items.clone(), &["MaxItems"], &["Marker"], 100);
        assert_eq!(p.items.len(), 100);
        assert_eq!(p.next_token.as_deref(), Some("100"));

        let p2 = paginate_json(
            &json!({"Marker": "100"}),
            items,
            &["MaxItems"],
            &["Marker"],
            100,
        );
        assert_eq!(p2.items.len(), 97);
        assert!(p2.next_token.is_none());
    }

    #[test]
    fn explicit_limit_from_params() {
        let mut params = HashMap::new();
        params.insert("MaxResults".to_string(), "5".to_string());
        let p = paginate_params(
            &params,
            (0..7).collect(),
            &["MaxResults"],
            &["NextToken"],
            100,
        );
        assert_eq!(p.items, vec![0, 1, 2, 3, 4]);
        assert_eq!(p.next_token.as_deref(), Some("5"));
    }

    #[test]
    fn out_of_range_token_is_empty() {
        let p = paginate_json(
            &json!({"nextToken": "999"}),
            (0..3).collect::<Vec<_>>(),
            &["maxResults"],
            &["nextToken"],
            10,
        );
        assert!(p.items.is_empty());
        assert!(p.next_token.is_none());
    }
}
