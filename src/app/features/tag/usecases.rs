use super::presenters::TagPresenter;
use super::repositories::TagRepository;
use crate::error::AppError;
use axum::response::Response;
use std::sync::Arc;

#[derive(Clone)]
pub struct TagUsecase {
    tag_repository: Arc<dyn TagRepository>,
    tag_presenter: Arc<dyn TagPresenter>,
}

impl TagUsecase {
    pub fn new(
        tag_repository: Arc<dyn TagRepository>,
        tag_presenter: Arc<dyn TagPresenter>,
    ) -> Self {
        Self {
            tag_repository,
            tag_presenter,
        }
    }

    pub async fn fetch_tags(&self) -> Result<Response, AppError> {
        let list = self.tag_repository.fetch_tags().await?;
        let res = self.tag_presenter.to_json(list);
        Ok(res)
    }
}
