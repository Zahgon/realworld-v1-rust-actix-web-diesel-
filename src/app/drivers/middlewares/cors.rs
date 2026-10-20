//! Cross-origin checks. FRONTEND_ORIGIN and any `*.rust-lang.org` origin may
//! make requests; no method is allowed cross-origin, so every preflight is
//! rejected with a plain-text 400.

use crate::constants::env_key;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use std::env;

#[derive(Clone)]
pub struct Cors {
    allowed_origins: Vec<HeaderValue>,
}

pub fn cors() -> Cors {
    let frontend_origin = env::var(env_key::FRONTEND_ORIGIN).unwrap_or_else(|_| "*".to_string());

    // A wildcard FRONTEND_ORIGIN leaves only the `*.rust-lang.org` rule.
    let allowed_origins = if frontend_origin == "*" {
        vec![]
    } else {
        HeaderValue::from_str(&frontend_origin).into_iter().collect()
    };
    Cors { allowed_origins }
}

enum CorsError {
    MissingOrigin,
    MissingRequestMethod,
    BadRequestMethod,
    OriginNotAllowed,
    MethodNotAllowed,
}

impl CorsError {
    fn message(&self) -> &'static str {
        match self {
            CorsError::MissingOrigin => "Request header `Origin` is required but was not provided",
            CorsError::MissingRequestMethod => {
                "Request header `Access-Control-Request-Method` is required but is missing"
            }
            CorsError::BadRequestMethod => {
                "Request header `Access-Control-Request-Method` has an invalid value"
            }
            CorsError::OriginNotAllowed => "Origin is not allowed to make this request",
            CorsError::MethodNotAllowed => "Requested method is not allowed",
        }
    }

    fn into_response(self) -> Response {
        let mut res = Response::new(Body::from(self.message()));
        *res.status_mut() = StatusCode::BAD_REQUEST;
        res
    }
}

impl Cors {
    fn is_allowed_origin(&self, origin: &HeaderValue) -> bool {
        self.allowed_origins.contains(origin) || origin.as_bytes().ends_with(b".rust-lang.org")
    }

    fn validate_origin(&self, headers: &HeaderMap) -> Result<(), CorsError> {
        match headers.get(header::ORIGIN) {
            Some(origin) if self.is_allowed_origin(origin) => Ok(()),
            Some(_) => Err(CorsError::OriginNotAllowed),
            None => Err(CorsError::MissingOrigin),
        }
    }

    fn preflight_error(&self, headers: &HeaderMap) -> CorsError {
        if let Err(err) = self.validate_origin(headers) {
            return err;
        }
        let request_method = headers
            .get(header::ACCESS_CONTROL_REQUEST_METHOD)
            .map(|value| value.to_str().ok().and_then(|m| Method::from_bytes(m.as_bytes()).ok()));
        match request_method {
            Some(Some(_)) => CorsError::MethodNotAllowed,
            Some(None) => CorsError::BadRequestMethod,
            None => CorsError::MissingRequestMethod,
        }
    }
}

pub async fn handle(State(cors): State<Cors>, req: Request, next: Next) -> Response {
    if req.method() == Method::OPTIONS
        && req.headers().contains_key(header::ACCESS_CONTROL_REQUEST_METHOD)
    {
        return cors.preflight_error(req.headers()).into_response();
    }

    let origin = req.headers().get(header::ORIGIN).cloned();
    if origin.is_some() {
        if let Err(err) = cors.validate_origin(req.headers()) {
            let mut res = err.into_response();
            add_vary_header(res.headers_mut());
            return res;
        }
    }

    let mut res = next.run(req).await;
    if let Some(origin) = origin {
        res.headers_mut().insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
    }
    add_vary_header(res.headers_mut());
    res
}

fn add_vary_header(headers: &mut HeaderMap) {
    const VARY: &str = "Origin, Access-Control-Request-Method, Access-Control-Request-Headers";
    let value = match headers.get(header::VARY) {
        Some(existing) => {
            let mut value = existing.as_bytes().to_vec();
            value.extend_from_slice(b", ");
            value.extend_from_slice(VARY.as_bytes());
            HeaderValue::from_bytes(&value).unwrap_or_else(|_| HeaderValue::from_static(VARY))
        }
        None => HeaderValue::from_static(VARY),
    };
    headers.insert(header::VARY, value);
}
