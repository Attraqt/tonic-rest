//! Query-string extractors with dotted nested object support.

use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use serde::de::DeserializeOwned;

/// Axum extractor for query strings that may contain dotted nested objects.
#[derive(Debug, Clone, Copy, Default)]
pub struct NestedQuery<T>(pub T);

/// Rejection returned when [`NestedQuery`] fails to deserialize the query string.
#[derive(Debug)]
pub struct NestedQueryRejection {
    message: String,
}

impl IntoResponse for NestedQueryRejection {
    fn into_response(self) -> Response {
        (
            StatusCode::BAD_REQUEST,
            format!("Failed to deserialize query string: {}", self.message),
        )
            .into_response()
    }
}

impl<S, T> FromRequestParts<S> for NestedQuery<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = NestedQueryRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let query = parts.uri.query().unwrap_or_default();
        let normalized_query = normalize_dotted_query(query)?;

        let value =
            serde_qs::from_str::<T>(&normalized_query).map_err(|err| NestedQueryRejection {
                message: err.to_string(),
            })?;

        Ok(Self(value))
    }
}

fn normalize_dotted_query(query: &str) -> Result<String, NestedQueryRejection> {
    let mut pairs = Vec::new();

    for (raw_key, value) in form_urlencoded::parse(query.as_bytes()) {
        let key = raw_key.as_ref();

        if key.is_empty() {
            continue;
        }

        let normalized_key =
            dotted_key_to_bracket_key(key).map_err(|message| NestedQueryRejection { message })?;

        pairs.push(format!(
            "{}={}",
            normalized_key,
            urlencoding::encode(value.as_ref())
        ));
    }

    Ok(pairs.join("&"))
}

fn dotted_key_to_bracket_key(key: &str) -> Result<String, String> {
    let mut parts = key.split('.');

    let Some(first) = parts.next() else {
        return Err("empty query key".to_string());
    };

    if first.is_empty() {
        return Err(format!("invalid dotted query key `{key}`"));
    }

    let mut normalized = first.to_string();

    for part in parts {
        if part.is_empty() {
            return Err(format!("invalid dotted query key `{key}`"));
        }

        normalized.push('[');
        normalized.push_str(part);
        normalized.push(']');
    }

    Ok(normalized)
}
