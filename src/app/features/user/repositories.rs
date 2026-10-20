use super::entities::UpdateUser;
use crate::app::features::follow::entities::{CreateFollow, DeleteFollow, Follow};
use crate::app::features::profile::entities::Profile;
use crate::app::features::user::entities::User;
use crate::error::AppError;
use crate::utils::db::DbPool;
use async_trait::async_trait;
use uuid::Uuid;

type Token = String;

#[async_trait]
pub trait UserRepository: Send + Sync + 'static {
    async fn signin(&self, email: &str, naive_password: &str) -> Result<(User, Token), AppError>;
    async fn signup(
        &self,
        email: &str,
        username: &str,
        naive_password: &str,
    ) -> Result<(User, Token), AppError>;
    async fn follow_user(
        &self,
        current_user: &User,
        target_username: &str,
    ) -> Result<Profile, AppError>;
    async fn unfollow_user(
        &self,
        current_user: &User,
        target_username: &str,
    ) -> Result<Profile, AppError>;
    async fn update(&self, user_id: Uuid, changeset: UpdateUser) -> Result<(User, Token), AppError>;
    async fn find(&self, user_id: Uuid) -> Result<User, AppError>;
}

#[derive(Clone)]
pub struct UserRepositoryImpl {
    pool: DbPool,
}

impl UserRepositoryImpl {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for UserRepositoryImpl {
    async fn signin(&self, email: &str, naive_password: &str) -> Result<(User, Token), AppError> {
        let mut conn = self.pool.acquire().await?;
        User::signin(&mut conn, email, naive_password).await
    }

    async fn signup(
        &self,
        email: &str,
        username: &str,
        naive_password: &str,
    ) -> Result<(User, Token), AppError> {
        let mut conn = self.pool.acquire().await?;
        User::signup(&mut conn, email, username, naive_password).await
    }

    async fn follow_user(
        &self,
        current_user: &User,
        target_username: &str,
    ) -> Result<Profile, AppError> {
        let mut conn = self.pool.acquire().await?;
        let followee = User::find_by_username(&mut conn, target_username).await?;

        Follow::create(
            &mut conn,
            &CreateFollow {
                follower_id: current_user.id,
                followee_id: followee.id,
            },
        )
        .await?;

        Ok(Profile {
            username: current_user.username.clone(),
            bio: current_user.bio.clone(),
            image: current_user.image.clone(),
            following: true,
        })
    }

    async fn unfollow_user(
        &self,
        current_user: &User,
        target_username: &str,
    ) -> Result<Profile, AppError> {
        let mut conn = self.pool.acquire().await?;
        let followee = User::find_by_username(&mut conn, target_username).await?;

        Follow::delete(
            &mut conn,
            &DeleteFollow {
                followee_id: followee.id,
                follower_id: current_user.id,
            },
        )
        .await?;

        Ok(Profile {
            username: current_user.username.clone(),
            bio: current_user.bio.clone(),
            image: current_user.image.clone(),
            following: false,
        })
    }

    async fn update(&self, user_id: Uuid, changeset: UpdateUser) -> Result<(User, Token), AppError> {
        let mut conn = self.pool.acquire().await?;
        let new_user = User::update(&mut conn, user_id, changeset).await?;
        let token = &new_user.generate_token()?;
        Ok((new_user, token.clone()))
    }

    async fn find(&self, user_id: Uuid) -> Result<User, AppError> {
        let mut conn = self.pool.acquire().await?;
        let user = User::find(&mut conn, user_id).await?;
        Ok(user)
    }
}
