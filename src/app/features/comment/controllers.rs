use super::request;
use crate::app::drivers::extractors::Json;
use crate::app::drivers::middlewares::auth::{self, CurrentUser};
use crate::app::drivers::middlewares::state::AppState;
use crate::utils::api::ApiResponse;
use crate::utils::uuid;
use axum::extract::{Path, State};

type ArticleIdSlug = String;
type CommentIdSlug = String;

pub async fn index(State(state): State<AppState>, req: CurrentUser) -> ApiResponse {
    let current_user = auth::get_current_user(&req).ok();
    state
        .di_container
        .comment_usecase
        .fetch_comments(&current_user)
        .await
}

pub async fn create(
    State(state): State<AppState>,
    req: CurrentUser,
    Path(article_title_slug): Path<ArticleIdSlug>,
    form: Json<request::CreateCommentRequest>,
) -> ApiResponse {
    let current_user = auth::get_current_user(&req)?;
    let body = form.comment.body.to_owned();
    state
        .di_container
        .comment_usecase
        .create_comment(body, article_title_slug, current_user)
        .await
}

pub async fn delete(
    State(state): State<AppState>,
    req: CurrentUser,
    Path((article_title_slug, comment_id)): Path<(ArticleIdSlug, CommentIdSlug)>,
) -> ApiResponse {
    let current_user = auth::get_current_user(&req)?;
    let comment_id = uuid::parse(&comment_id)?;
    state
        .di_container
        .comment_usecase
        .delete_comment(&article_title_slug, comment_id, current_user.id)
        .await
}
