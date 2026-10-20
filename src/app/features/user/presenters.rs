use crate::{app::features::user::entities::User, error::AppError};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::convert::From;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UserResponse {
    pub user: AuthUser,
}

impl From<(User, String)> for UserResponse {
    fn from((user, token): (User, String)) -> Self {
        // REF: https://gothinkster.github.io/realworld/docs/specs/backend-specs/api-response-format/#users-for-authentication
        Self {
            user: AuthUser {
                email: user.email,
                token,
                username: user.username,
                bio: user.bio,
                image: user.image,
            },
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AuthUser {
    pub email: String,
    pub token: String,
    pub username: String,
    pub bio: Option<String>,
    pub image: Option<String>,
}

pub trait UserPresenter: Send + Sync + 'static {
    fn to_json(&self, user: User, token: String) -> Response;
    fn to_auth_middleware(&self, maybe_uesr: Result<User, AppError>) -> Result<User, &'static str>;
}

#[derive(Clone)]
pub struct UserPresenterImpl {}
impl UserPresenterImpl {
    pub fn new() -> Self {
        Self {}
    }
}
impl UserPresenter for UserPresenterImpl {
    fn to_json(&self, user: User, token: String) -> Response {
        let res_model = UserResponse::from((user, token));
        Json(res_model).into_response()
    }

    fn to_auth_middleware(&self, maybe_user: Result<User, AppError>) -> Result<User, &'static str> {
        maybe_user.map_err(|_err| "Cannot find auth user")
    }
}
