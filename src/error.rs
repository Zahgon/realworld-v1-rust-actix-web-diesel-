use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use bcrypt::BcryptError;
use jsonwebtoken::errors::{Error as JwtError, ErrorKind as JwtErrorKind};
use serde_json::json;
use serde_json::Value as JsonValue;
use sqlx::error::{Error as SqlxError, ErrorKind as DatabaseErrorKind};
use sqlx::postgres::PgDatabaseError;
use std::convert::From;
use thiserror::Error;
use uuid::Error as UuidError;

#[derive(Error, Debug)]
pub enum AppError {
    // 401
    #[error("Unauthorized: {}", _0)]
    Unauthorized(JsonValue),

    // 403
    #[error("Forbidden: {}", _0)]
    Forbidden(JsonValue),

    // 404
    #[error("Not Found: {}", _0)]
    NotFound(JsonValue),

    // 422
    #[error("Unprocessable Entity: {}", _0)]
    UnprocessableEntity(JsonValue),

    // 500
    #[error("Internal Server Error")]
    InternalServerError,
}

impl AppError {
    pub fn status_code(&self) -> StatusCode {
        match *self {
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::UnprocessableEntity(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        match self {
            AppError::Unauthorized(msg)
            | AppError::Forbidden(msg)
            | AppError::NotFound(msg)
            | AppError::UnprocessableEntity(msg) => (status, Json(msg)).into_response(),
            AppError::InternalServerError => {
                (status, Json("Internal Server Error")).into_response()
            }
        }
    }
}

impl From<BcryptError> for AppError {
    fn from(_err: BcryptError) -> Self {
        AppError::InternalServerError
    }
}

impl From<JwtError> for AppError {
    fn from(err: JwtError) -> Self {
        match err.kind() {
            JwtErrorKind::InvalidToken => AppError::Unauthorized(json!({
                "error": "Token is invalid"
            })),
            JwtErrorKind::InvalidIssuer => AppError::Unauthorized(json!({
                "error": "Issuer is invalid",
            })),
            _ => AppError::Unauthorized(json!({
                "error": "An issue was found with the token provided",
            })),
        }
    }
}

impl From<SqlxError> for AppError {
    fn from(err: SqlxError) -> Self {
        match err {
            SqlxError::Database(info) => {
                if let DatabaseErrorKind::UniqueViolation = info.kind() {
                    let message = info
                        .try_downcast_ref::<PgDatabaseError>()
                        .and_then(|pg| pg.detail())
                        .unwrap_or_else(|| info.message())
                        .to_string();
                    AppError::UnprocessableEntity(json!({ "error": message }))
                } else {
                    AppError::InternalServerError
                }
            }
            SqlxError::RowNotFound => {
                AppError::NotFound(json!({ "error": "requested record was not found" }))
            }
            _ => AppError::InternalServerError,
        }
    }
}

impl From<UuidError> for AppError {
    fn from(_err: UuidError) -> Self {
        AppError::NotFound(json!({"error":"Uuid is invalid."}))
    }
}
