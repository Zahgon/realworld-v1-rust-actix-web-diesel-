use super::presenters::CommentPresenter;
use super::repositories::CommentRepository;
use crate::app::features::user::entities::User;
use crate::error::AppError;
use axum::response::Response;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct CommentUsecase {
    comment_repository: Arc<dyn CommentRepository>,
    comment_presenter: Arc<dyn CommentPresenter>,
}

impl CommentUsecase {
    pub fn new(
        comment_repository: Arc<dyn CommentRepository>,
        comment_presenter: Arc<dyn CommentPresenter>,
    ) -> Self {
        Self {
            comment_repository,
            comment_presenter,
        }
    }

    pub async fn fetch_comments(&self, user: &Option<User>) -> Result<Response, AppError> {
        let result = self.comment_repository.fetch_comments(user).await?;
        let res = self.comment_presenter.to_multi_json(result);
        Ok(res)
    }

    pub async fn create_comment(
        &self,
        body: String,
        article_title_slug: String,
        author: User,
    ) -> Result<Response, AppError> {
        let result = self
            .comment_repository
            .create_comment(body, article_title_slug, author)
            .await?;
        let res = self.comment_presenter.to_single_json(result);
        Ok(res)
    }

    pub async fn delete_comment(
        &self,
        article_title_slug: &str,
        comment_id: Uuid,
        author_id: Uuid,
    ) -> Result<Response, AppError> {
        let _ = self
            .comment_repository
            .delete_comment(article_title_slug, comment_id, author_id)
            .await;
        let res = self.comment_presenter.to_http_res();
        Ok(res)
    }
}
