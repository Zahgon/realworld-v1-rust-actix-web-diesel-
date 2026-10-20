use crate::app::drivers::middlewares::state::AppState;
use crate::utils::api::ApiResponse;
use axum::extract::State;

pub async fn index(State(state): State<AppState>) -> ApiResponse {
    state.di_container.tag_usecase.fetch_tags().await
}
