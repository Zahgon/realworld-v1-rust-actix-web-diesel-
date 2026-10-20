use super::entities::Profile;
use crate::app::features::user::entities::User;
use crate::error::AppError;
use crate::utils::db::DbPool;
use async_trait::async_trait;

#[async_trait]
pub trait ProfileRepository: Send + Sync + 'static {
    async fn fetch_profile_by_name(
        &self,
        current_user: &User,
        username: &str,
    ) -> Result<Profile, AppError>;
}

#[derive(Clone)]
pub struct ProfileRepositoryImpl {
    pool: DbPool,
}

impl ProfileRepositoryImpl {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProfileRepository for ProfileRepositoryImpl {
    async fn fetch_profile_by_name(
        &self,
        current_user: &User,
        username: &str,
    ) -> Result<Profile, AppError> {
        let mut conn = self.pool.acquire().await?;
        let profile = {
            let followee = User::find_by_username(&mut conn, username).await?;
            current_user.fetch_profile(&mut conn, &followee.id).await?
        };
        Ok(profile)
    }
}
