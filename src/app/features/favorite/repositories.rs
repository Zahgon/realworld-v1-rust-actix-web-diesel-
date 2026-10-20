use super::entities::{CreateFavorite, DeleteFavorite, Favorite};
use crate::app::features::article::entities::{Article, FetchBySlugAndAuthorId};
use crate::app::features::user::entities::User;
use crate::error::AppError;
use crate::utils::db::DbPool;
use async_trait::async_trait;

#[async_trait]
pub trait FavoriteRepository: Send + Sync + 'static {
    async fn favorite_article(
        &self,
        user: User,
        article_title_slug: String,
    ) -> Result<Article, AppError>;
    async fn unfavorite_article(
        &self,
        user: User,
        article_title_slug: String,
    ) -> Result<Article, AppError>;
}

#[derive(Clone)]
pub struct FavoriteRepositoryImpl {
    pool: DbPool,
}

impl FavoriteRepositoryImpl {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl FavoriteRepository for FavoriteRepositoryImpl {
    async fn favorite_article(
        &self,
        user: User,
        article_title_slug: String,
    ) -> Result<Article, AppError> {
        let mut conn = self.pool.acquire().await?;

        let article = Article::fetch_by_slug_and_author_id(
            &mut conn,
            &FetchBySlugAndAuthorId {
                slug: article_title_slug,
                author_id: user.id,
            },
        )
        .await?;
        Favorite::create(
            &mut conn,
            &CreateFavorite {
                user_id: user.id,
                article_id: article.id,
            },
        )
        .await?;

        Ok(article)
    }

    async fn unfavorite_article(
        &self,
        user: User,
        article_title_slug: String,
    ) -> Result<Article, AppError> {
        let mut conn = self.pool.acquire().await?;
        let article = Article::fetch_by_slug_and_author_id(
            &mut conn,
            &FetchBySlugAndAuthorId {
                slug: article_title_slug,
                author_id: user.id,
            },
        )
        .await?;
        Favorite::delete(
            &mut conn,
            &DeleteFavorite {
                user_id: user.id,
                article_id: article.id,
            },
        )
        .await?;
        Ok(article)
    }
}
