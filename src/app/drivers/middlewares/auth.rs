use crate::app::drivers::middlewares::state::AppState;
use crate::app::features::user::entities::User;
use crate::constants;
use crate::error::AppError;
use crate::utils::token;
use axum::async_trait;
use axum::extract::{FromRequestParts, Request, State};
use axum::http::{request::Parts, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use std::convert::Infallible;
use uuid::Uuid;

// Runs ahead of CORS and routing for every request: a request that is neither
// OPTIONS nor on SKIP_AUTH_ROUTES must carry a token for an existing user, or
// it is answered 401 with an empty body.
pub async fn authentication(State(state): State<AppState>, mut req: Request, next: Next) -> Response {
    let is_verified = if should_skip_auth(&req) {
        true
    } else {
        set_auth_user(&state, &mut req).await
    };
    if is_verified {
        next.run(req).await
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}

fn should_skip_auth(req: &Request) -> bool {
    let method = req.method();
    if Method::OPTIONS == *method {
        return true;
    }

    SKIP_AUTH_ROUTES
        .iter()
        .any(|route| route.matches_path_and_method(req.uri().path(), req.method()))
}

const TOKEN_IDENTIFIER: &str = "Token";

async fn set_auth_user(state: &AppState, req: &mut Request) -> bool {
    match fetch_user(state, get_user_id_from_header(req)).await {
        Ok(user) => {
            req.extensions_mut().insert(user);
            true
        }
        Err(err_msg) => {
            info!("Cannot fetch user {}", err_msg);
            false
        }
    }
}

async fn fetch_user(
    state: &AppState,
    user_id: Result<Uuid, &'static str>,
) -> Result<User, &'static str> {
    state.di_container.user_usecase.find_auth_user(user_id?).await
}

fn get_user_id_from_header(req: &Request) -> Result<Uuid, &'static str> {
    req.headers()
        .get(constants::AUTHORIZATION)
        .ok_or("Cannot find authrization key-value in req header")
        .and_then(|auth_header| auth_header.to_str().map_err(|_err| "Cannot stringify"))
        .and_then(|auth_str| {
            if auth_str.starts_with(TOKEN_IDENTIFIER) {
                Ok(auth_str)
            } else {
                Err("Invalid token convention")
            }
        })
        .map(|auth_str| auth_str[6..auth_str.len()].trim())
        .and_then(|token| token::decode(token).map_err(|_err| "Cannot decode token."))
        .map(|token| token.claims.user_id)
}

/// The user the authentication middleware attached to the request, if any.
pub struct CurrentUser(Option<User>);

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for CurrentUser {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(CurrentUser(parts.extensions.get::<User>().cloned()))
    }
}

pub fn get_current_user(req: &CurrentUser) -> Result<User, AppError> {
    req.0.as_ref().map(|user| user.to_owned()).ok_or_else(|| {
        AppError::Unauthorized(json!({"error": "Unauthrized user. Need auth token on header."}))
    })
}

struct SkipAuthRoute {
    path: &'static str,
    method: Method,
}

impl SkipAuthRoute {
    fn matches_path_and_method(&self, path: &str, method: &Method) -> bool {
        self.matches_path(path) && self.matches_method(method)
    }

    fn matches_path(&self, path: &str) -> bool {
        let expect_path = self.path.split('/').collect::<Vec<_>>();
        let this_path = path.split('/').collect::<Vec<_>>();
        if expect_path.len() != this_path.len() {
            return false;
        };
        let path_set = expect_path.iter().zip(this_path.iter());
        for (expect_path, this_path) in path_set {
            if SkipAuthRoute::is_slug_path(expect_path) {
                continue;
            }
            if expect_path != this_path {
                return false;
            }
        }
        true
    }

    fn matches_method(&self, method: &Method) -> bool {
        self.method == method
    }

    fn is_slug_path(text: &str) -> bool {
        let first = text.chars().next().unwrap_or(' ');
        let last = text.chars().last().unwrap_or(' ');
        first == '{' && last == '}'
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Method;
    #[test]
    fn is_match_path_and_method_test() {
        let route = SkipAuthRoute {
            path: "/api/healthcheck",
            method: Method::GET,
        };
        assert!(route.matches_path_and_method("/api/healthcheck", &Method::GET));

        let route = SkipAuthRoute {
            path: "/api/{this-is-slug}/healthcheck",
            method: Method::POST,
        };
        assert!(route.matches_path_and_method("/api/1234/healthcheck", &Method::POST));
    }
}

const SKIP_AUTH_ROUTES: [SkipAuthRoute; 6] = [
    SkipAuthRoute {
        path: "/api/healthcheck",
        method: Method::GET,
    },
    SkipAuthRoute {
        path: "/api/tags",
        method: Method::GET,
    },
    SkipAuthRoute {
        path: "/api/users",
        method: Method::POST,
    },
    SkipAuthRoute {
        path: "/api/users/login",
        method: Method::POST,
    },
    SkipAuthRoute {
        path: "/api/articles",
        method: Method::GET,
    },
    SkipAuthRoute {
        path: "/api/articles/{article_title_slug}/comments",
        method: Method::GET,
    },
];
