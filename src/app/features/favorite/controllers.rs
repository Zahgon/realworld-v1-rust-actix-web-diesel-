use crate::app::drivers::middlewares::auth::{self, CurrentUser};
use crate::app::drivers::middlewares::state::AppState;
use crate::utils::api::ApiResponse;
use axum::extract::{Path, State};

type ArticleIdSlug = String;

pub async fn favorite(
    State(state): State<AppState>,
    req: CurrentUser,
    Path(article_title_slug): Path<ArticleIdSlug>,
) -> ApiResponse {
    let current_user = auth::get_current_user(&req)?;
    state
        .di_container
        .favorite_usecase
        .favorite_article(current_user, article_title_slug)
        .await
}

pub async fn unfavorite(
    State(state): State<AppState>,
    req: CurrentUser,
    Path(article_title_slug): Path<ArticleIdSlug>,
) -> ApiResponse {
    let current_user = auth::get_current_user(&req)?;
    state
        .di_container
        .favorite_usecase
        .unfavorite_article(current_user, article_title_slug)
        .await
}
