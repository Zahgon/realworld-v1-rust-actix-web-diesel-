use super::{
    requests,
    usecases::{
        CreateArticleUsecaseInput, DeleteArticleUsecaseInput, FetchArticlesUsecaseInput,
        UpdateArticleUsecaseInput,
    },
};
use crate::app::drivers::extractors::{Json, Query};
use crate::app::drivers::middlewares::auth::{self, CurrentUser};
use crate::app::drivers::middlewares::state::AppState;
use crate::utils::api::ApiResponse;
use axum::extract::{Path, State};
use serde::Deserialize;

type ArticleTitleSlug = String;

#[derive(Deserialize)]
pub struct ArticlesListQueryParameter {
    tag: Option<String>,
    author: Option<String>,
    favorited: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

pub async fn index(
    State(state): State<AppState>,
    params: Query<ArticlesListQueryParameter>,
) -> ApiResponse {
    let offset = std::cmp::min(params.offset.to_owned().unwrap_or(0), 100);
    let limit = params.limit.unwrap_or(20);
    state
        .di_container
        .article_usecase
        .fetch_articles(FetchArticlesUsecaseInput {
            tag: params.tag.clone(),
            author: params.author.clone(),
            favorited: params.favorited.clone(),
            offset,
            limit,
        })
        .await
}

#[derive(Deserialize)]
pub struct FeedQueryParameter {
    limit: Option<i64>,
    offset: Option<i64>,
}

pub async fn feed(
    State(state): State<AppState>,
    req: CurrentUser,
    params: Query<FeedQueryParameter>,
) -> ApiResponse {
    let current_user = auth::get_current_user(&req)?;
    let offset = std::cmp::min(params.offset.to_owned().unwrap_or(0), 100);
    let limit = params.limit.unwrap_or(20);
    state
        .di_container
        .article_usecase
        .fetch_following_articles(current_user, offset, limit)
        .await
}

pub async fn show(
    State(state): State<AppState>,
    Path(article_title_slug): Path<ArticleTitleSlug>,
) -> ApiResponse {
    state
        .di_container
        .article_usecase
        .fetch_article_by_slug(article_title_slug)
        .await
}

pub async fn create(
    State(state): State<AppState>,
    req: CurrentUser,
    form: Json<requests::CreateArticleRequest>,
) -> ApiResponse {
    let current_user = auth::get_current_user(&req)?;
    state
        .di_container
        .article_usecase
        .create_article(CreateArticleUsecaseInput {
            title: form.article.title.clone(),
            description: form.article.description.clone(),
            body: form.article.body.clone(),
            tag_name_list: form.article.tag_list.to_owned(),
            current_user,
        })
        .await
}

pub async fn update(
    State(state): State<AppState>,
    req: CurrentUser,
    Path(article_title_slug): Path<ArticleTitleSlug>,
    form: Json<requests::UpdateArticleRequest>,
) -> ApiResponse {
    let current_user = auth::get_current_user(&req)?;
    let title = form.article.title.clone();
    let description = form.article.description.clone();
    let body = form.article.body.clone();
    state
        .di_container
        .article_usecase
        .update_article(UpdateArticleUsecaseInput {
            current_user,
            article_title_slug,
            title,
            description,
            body,
        })
        .await
}

pub async fn delete(
    State(state): State<AppState>,
    req: CurrentUser,
    Path(article_title_slug): Path<ArticleTitleSlug>,
) -> ApiResponse {
    let current_user = auth::get_current_user(&req)?;
    state
        .di_container
        .article_usecase
        .delete_article(DeleteArticleUsecaseInput {
            author_id: current_user.id,
            slug: article_title_slug,
        })
        .await
}
