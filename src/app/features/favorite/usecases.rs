use super::presenters::FavoritePresenter;
use super::repositories::FavoriteRepository;
use crate::app::features::article::repositories::{ArticleRepository, FetchArticleRepositoryInput};
use crate::app::features::user::entities::User;
use crate::error::AppError;
use axum::response::Response;
use std::sync::Arc;

#[derive(Clone)]
pub struct FavoriteUsecase {
    favorite_repository: Arc<dyn FavoriteRepository>,
    favorite_presenter: Arc<dyn FavoritePresenter>,
    article_repository: Arc<dyn ArticleRepository>,
}

impl FavoriteUsecase {
    pub fn new(
        favorite_repository: Arc<dyn FavoriteRepository>,
        favorite_presenter: Arc<dyn FavoritePresenter>,
        article_repository: Arc<dyn ArticleRepository>,
    ) -> Self {
        Self {
            favorite_repository,
            favorite_presenter,
            article_repository,
        }
    }

    pub async fn favorite_article(
        &self,
        user: User,
        article_title_slug: String,
    ) -> Result<Response, AppError> {
        let article = self
            .favorite_repository
            .favorite_article(user.clone(), article_title_slug)
            .await?;

        let result = self
            .article_repository
            .fetch_article(&FetchArticleRepositoryInput {
                article_id: article.id,
                current_user: user,
            })
            .await?;
        let res = self.favorite_presenter.to_single_json(result);
        Ok(res)
    }

    pub async fn unfavorite_article(
        &self,
        user: User,
        article_title_slug: String,
    ) -> Result<Response, AppError> {
        let article = self
            .favorite_repository
            .unfavorite_article(user.clone(), article_title_slug)
            .await?;

        let result = self
            .article_repository
            .fetch_article(&FetchArticleRepositoryInput {
                article_id: article.id,
                current_user: user,
            })
            .await?;

        let res = self.favorite_presenter.to_single_json(result);
        Ok(res)
    }
}
