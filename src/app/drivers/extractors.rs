//! Body and query extractors whose rejections are part of the API's contract:
//! `Json` accepts any content type whose subtype or suffix is `json` and
//! rejects a declared length over the limit before reading the body; both
//! answer failures with the plain-text messages below.

use axum::async_trait;
use axum::body::to_bytes;
use axum::extract::{FromRequest, FromRequestParts, Request};
use axum::http::{header, request::Parts, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::de::DeserializeOwned;
use std::ops::Deref;

/// Largest accepted JSON body, in bytes.
const JSON_LIMIT: usize = 2_097_152;

pub struct Json<T>(pub T);

impl<T> Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

#[async_trait]
impl<T, S> FromRequest<S> for Json<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        if !is_json(req.headers()) {
            return Err(plain_text(StatusCode::BAD_REQUEST, "Content type error".to_string()));
        }

        let length = req
            .headers()
            .get(header::CONTENT_LENGTH)
            .and_then(|l| l.to_str().ok())
            .and_then(|s| s.parse::<usize>().ok());
        if let Some(length) = length {
            if length > JSON_LIMIT {
                return Err(plain_text(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    format!(
                        "JSON payload ({length} bytes) is larger than allowed (limit: {JSON_LIMIT} bytes)."
                    ),
                ));
            }
        }

        let body = to_bytes(req.into_body(), JSON_LIMIT).await.map_err(|_err| {
            plain_text(
                StatusCode::PAYLOAD_TOO_LARGE,
                format!("JSON payload has exceeded limit ({JSON_LIMIT} bytes)."),
            )
        })?;

        serde_json::from_slice::<T>(&body)
            .map(Json)
            .map_err(|err| plain_text(StatusCode::BAD_REQUEST, format!("Json deserialize error: {err}")))
    }
}

fn is_json(headers: &HeaderMap) -> bool {
    let mime = match headers.get(header::CONTENT_TYPE) {
        Some(value) => match value.to_str().ok().and_then(|s| s.parse::<mime::Mime>().ok()) {
            Some(mime) => mime,
            None => return false,
        },
        None => return false,
    };
    mime.subtype() == mime::JSON || mime.suffix() == Some(mime::JSON)
}

pub struct Query<T>(pub T);

impl<T> Deref for Query<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

#[async_trait]
impl<T, S> FromRequestParts<S> for Query<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let query = parts.uri.query().unwrap_or_default();
        serde_urlencoded::from_str::<T>(query)
            .map(Query)
            .map_err(|err| plain_text(StatusCode::BAD_REQUEST, format!("Query deserialize error: {err}")))
    }
}

fn plain_text(status: StatusCode, message: String) -> Response {
    (status, message).into_response()
}
