use super::entities::Tag;
use crate::error::AppError;
use crate::utils::db::DbPool;
use async_trait::async_trait;

#[async_trait]
pub trait TagRepository: Send + Sync + 'static {
    async fn fetch_tags(&self) -> Result<Vec<Tag>, AppError>;
}

#[derive(Clone)]
pub struct TagRepositoryImpl {
    pool: DbPool,
}

impl TagRepositoryImpl {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TagRepository for TagRepositoryImpl {
    async fn fetch_tags(&self) -> Result<Vec<Tag>, AppError> {
        let mut conn = self.pool.acquire().await?;
        Tag::fetch(&mut conn).await
    }
}
