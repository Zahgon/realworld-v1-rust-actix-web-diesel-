use super::entities::{Comment, CreateComment};
use crate::{
    app::features::{
        article::entities::{Article, FetchBySlugAndAuthorId},
        profile::entities::Profile,
        user::entities::User,
    },
    error::AppError,
    utils::db::DbPool,
};
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait CommentRepository: Send + Sync + 'static {
    async fn fetch_comments(
        &self,
        current_user: &Option<User>,
    ) -> Result<Vec<(Comment, Profile)>, AppError>;

    async fn create_comment(
        &self,
        body: String,
        article_title_slug: String,
        author: User,
    ) -> Result<(Comment, Profile), AppError>;

    async fn delete_comment(
        &self,
        article_title_slug: &str,
        comment_id: Uuid,
        author_id: Uuid,
    ) -> Result<(), AppError>;
}

#[derive(Clone)]
pub struct CommentRepositoryImpl {
    pool: DbPool,
}

impl CommentRepositoryImpl {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CommentRepository for CommentRepositoryImpl {
    async fn fetch_comments(
        &self,
        current_user: &Option<User>,
    ) -> Result<Vec<(Comment, Profile)>, AppError> {
        let mut conn = self.pool.acquire().await?;

        let comments = Comment::fetch_all_with_author(&mut conn).await?;

        let mut list = Vec::with_capacity(comments.len());
        for (comment, user) in comments.iter() {
            // TODO: avoid N+1. Write one query to fetch all data somehow.
            let profile = user.to_profile(&mut conn, current_user).await;

            // TODO: avoid copy
            list.push((comment.to_owned(), profile));
        }

        Ok(list)
    }

    async fn create_comment(
        &self,
        body: String,
        article_title_slug: String,
        author: User,
    ) -> Result<(Comment, Profile), AppError> {
        let mut conn = self.pool.acquire().await?;

        let article = Article::fetch_by_slug_and_author_id(
            &mut conn,
            &FetchBySlugAndAuthorId {
                slug: article_title_slug,
                author_id: author.id,
            },
        )
        .await?;
        let comment = Comment::create(
            &mut conn,
            &CreateComment {
                body,
                author_id: author.id,
                article_id: article.id.to_owned(),
            },
        )
        .await?;
        let profile = author.fetch_profile(&mut conn, &author.id).await?;
        Ok((comment, profile))
    }

    async fn delete_comment(
        &self,
        article_title_slug: &str,
        comment_id: Uuid,
        author_id: Uuid,
    ) -> Result<(), AppError> {
        let mut conn = self.pool.acquire().await?;
        let _ = Comment::delete(&mut conn, (&comment_id, &author_id, article_title_slug)).await;
        Ok(())
    }
}
