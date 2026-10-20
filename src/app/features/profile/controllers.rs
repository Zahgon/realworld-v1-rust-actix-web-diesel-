use crate::app::drivers::middlewares::auth::{self, CurrentUser};
use crate::app::drivers::middlewares::state::AppState;
use crate::utils::api::ApiResponse;
use axum::extract::{Path, State};

type UsernameSlug = String;

pub async fn show(
    State(state): State<AppState>,
    req: CurrentUser,
    Path(username): Path<UsernameSlug>,
) -> ApiResponse {
    let current_user = auth::get_current_user(&req)?;
    state
        .di_container
        .profile_usecase
        .fetch_profile_by_name(&current_user, &username)
        .await
}

pub async fn follow(
    State(state): State<AppState>,
    req: CurrentUser,
    Path(target_username): Path<UsernameSlug>,
) -> ApiResponse {
    let current_user = auth::get_current_user(&req)?;
    state
        .di_container
        .profile_usecase
        .follow_user(&current_user, &target_username)
        .await
}

pub async fn unfollow(
    State(state): State<AppState>,
    req: CurrentUser,
    Path(target_username): Path<UsernameSlug>,
) -> ApiResponse {
    let current_user = auth::get_current_user(&req)?;
    state
        .di_container
        .profile_usecase
        .unfollow_user(&current_user, &target_username)
        .await
}
