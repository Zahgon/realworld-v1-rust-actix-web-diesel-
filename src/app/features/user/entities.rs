use crate::app::features::profile::entities::Profile;
use crate::error::AppError;
use crate::utils::{hasher, token};
use chrono::prelude::*;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgConnection, PgRow};
use sqlx::{FromRow, Row};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub password: String,
    pub bio: Option<String>,
    pub image: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

type Token = String;

impl User {
    pub const COLUMNS: &'static str = "users.id, users.email, users.username, users.password, \
        users.bio, users.image, users.created_at, users.updated_at";

    /// Reads a user from `row` starting at column `start` (joined rows put the
    /// user's columns after another table's).
    pub fn from_row_at(row: &PgRow, start: usize) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get(start)?,
            email: row.try_get(start + 1)?,
            username: row.try_get(start + 2)?,
            password: row.try_get(start + 3)?,
            bio: row.try_get(start + 4)?,
            image: row.try_get(start + 5)?,
            created_at: row.try_get(start + 6)?,
            updated_at: row.try_get(start + 7)?,
        })
    }
}

impl<'r> FromRow<'r, PgRow> for User {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Self::from_row_at(row, 0)
    }
}

impl User {
    pub async fn signup(
        conn: &mut PgConnection,
        email: &str,
        username: &str,
        naive_password: &str,
    ) -> Result<(User, Token), AppError> {
        let hashed_password = hasher::hash_password(naive_password).await?;

        let user = sqlx::query_as::<_, User>(&format!(
            "INSERT INTO users (email, username, password) VALUES ($1, $2, $3) RETURNING {}",
            Self::COLUMNS
        ))
        .bind(email)
        .bind(username)
        .bind(&hashed_password)
        .fetch_one(&mut *conn)
        .await?;

        let token = user.generate_token()?;
        Ok((user, token))
    }

    pub async fn signin(
        conn: &mut PgConnection,
        email: &str,
        naive_password: &str,
    ) -> Result<(User, Token), AppError> {
        let user = sqlx::query_as::<_, User>(&format!(
            "SELECT {} FROM users WHERE users.email = $1 LIMIT 1",
            Self::COLUMNS
        ))
        .bind(email)
        .fetch_one(&mut *conn)
        .await?;
        hasher::verify(naive_password, &user.password).await?;
        let token = user.generate_token()?;
        Ok((user, token))
    }

    pub async fn find(conn: &mut PgConnection, id: Uuid) -> Result<Self, AppError> {
        let user = sqlx::query_as::<_, User>(&format!(
            "SELECT {} FROM users WHERE users.id = $1 LIMIT 1",
            Self::COLUMNS
        ))
        .bind(id)
        .fetch_one(&mut *conn)
        .await?;
        Ok(user)
    }

    pub async fn update(
        conn: &mut PgConnection,
        user_id: Uuid,
        changeset: UpdateUser,
    ) -> Result<Self, AppError> {
        // Only the fields present in the changeset are written; with none
        // present there is nothing to save, which the API answers with a 500.
        let fields = [
            ("email", changeset.email),
            ("username", changeset.username),
            ("password", changeset.password),
            ("image", changeset.image),
            ("bio", changeset.bio),
        ];
        let mut assignments = Vec::new();
        let mut values = Vec::new();
        for (column, value) in fields {
            if let Some(value) = value {
                values.push(value);
                assignments.push(format!("{column} = ${}", values.len()));
            }
        }
        if assignments.is_empty() {
            return Err(AppError::InternalServerError);
        }

        let sql = format!(
            "UPDATE users SET {} WHERE users.id = ${} RETURNING {}",
            assignments.join(", "),
            values.len() + 1,
            Self::COLUMNS
        );
        let mut query = sqlx::query_as::<_, User>(&sql);
        for value in values {
            query = query.bind(value);
        }
        let user = query.bind(user_id).fetch_one(&mut *conn).await?;
        Ok(user)
    }

    pub async fn find_by_username(conn: &mut PgConnection, username: &str) -> Result<Self, AppError> {
        let user = sqlx::query_as::<_, User>(&format!(
            "SELECT {} FROM users WHERE users.username = $1 LIMIT 1",
            Self::COLUMNS
        ))
        .bind(username)
        .fetch_one(&mut *conn)
        .await?;
        Ok(user)
    }

    pub async fn is_following(&self, conn: &mut PgConnection, followee_id: &Uuid) -> bool {
        let follow = sqlx::query(
            "SELECT follows.followee_id FROM follows \
             WHERE follows.followee_id = $1 AND follows.follower_id = $2",
        )
        .bind(followee_id)
        .bind(self.id)
        .fetch_optional(&mut *conn)
        .await;
        matches!(follow, Ok(Some(_)))
    }
}

impl User {
    pub fn generate_token(&self) -> Result<String, AppError> {
        let now = Utc::now().timestamp();
        let token = token::generate(self.id, now)?;
        Ok(token)
    }

    pub async fn fetch_favorited_article_ids(
        &self,
        conn: &mut PgConnection,
    ) -> Result<Vec<Uuid>, AppError> {
        let favorited_article_ids = sqlx::query_scalar::<_, Uuid>(
            "SELECT favorites.article_id FROM favorites WHERE favorites.user_id = $1",
        )
        .bind(self.id)
        .fetch_all(&mut *conn)
        .await?;
        Ok(favorited_article_ids)
    }

    pub async fn fetch_profile(
        &self,
        conn: &mut PgConnection,
        folowee_id: &Uuid,
    ) -> Result<Profile, AppError> {
        let is_following = self.is_following(conn, folowee_id).await;
        let profile = Profile {
            username: self.username.to_owned(),
            bio: self.bio.to_owned(),
            image: self.image.to_owned(),
            following: is_following,
        };
        Ok(profile)
    }

    pub async fn to_profile(&self, conn: &mut PgConnection, current_user: &Option<User>) -> Profile {
        let user = self;
        let following = match current_user {
            Some(current_user) => current_user.is_following(conn, &user.id).await,
            None => false,
        };

        Profile {
            username: user.username.to_owned(),
            bio: user.bio.to_owned(),
            image: user.image.to_owned(),
            following,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct UpdateUser {
    pub email: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub image: Option<String>,
    pub bio: Option<String>,
}
