use crate::error::AppError;
use axum::response::Response;

pub type ApiResponse = Result<Response, AppError>;
