use super::entities::UpdateUser;
use super::requests;
use crate::app::drivers::extractors::Json;
use crate::app::drivers::middlewares::auth::{self, CurrentUser};
use crate::app::drivers::middlewares::state::AppState;
use crate::utils::api::ApiResponse;
use axum::extract::State;

pub async fn signin(State(state): State<AppState>, form: Json<requests::Signin>) -> ApiResponse {
    state
        .di_container
        .user_usecase
        .signin(&form.user.email, &form.user.password)
        .await
}

pub async fn signup(State(state): State<AppState>, form: Json<requests::Signup>) -> ApiResponse {
    state
        .di_container
        .user_usecase
        .signup(&form.user.email, &form.user.username, &form.user.password)
        .await
}

pub async fn me(State(state): State<AppState>, req: CurrentUser) -> ApiResponse {
    let current_user = auth::get_current_user(&req)?;
    state.di_container.user_usecase.get_token(&current_user)
}

pub async fn update(
    State(state): State<AppState>,
    req: CurrentUser,
    form: Json<requests::Update>,
) -> ApiResponse {
    let current_user = auth::get_current_user(&req)?;
    state
        .di_container
        .user_usecase
        .update_user(
            current_user.id,
            UpdateUser {
                email: form.user.email.clone(),
                username: form.user.username.clone(),
                password: form.user.password.clone(),
                image: form.user.image.clone(),
                bio: form.user.bio.clone(),
            },
        )
        .await
}
