use crate::error::AppError;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgConnection;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Favorite {
    pub id: Uuid,
    pub article_id: Uuid,
    pub user_id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl Favorite {
    pub async fn create(conn: &mut PgConnection, record: &CreateFavorite) -> Result<usize, AppError> {
        let result = sqlx::query("INSERT INTO favorites (user_id, article_id) VALUES ($1, $2)")
            .bind(record.user_id)
            .bind(record.article_id)
            .execute(&mut *conn)
            .await?;
        Ok(result.rows_affected() as usize)
    }

    pub async fn delete(
        conn: &mut PgConnection,
        DeleteFavorite {
            user_id,
            article_id,
        }: &DeleteFavorite,
    ) -> Result<usize, AppError> {
        let result = sqlx::query(
            "DELETE FROM favorites WHERE favorites.user_id = $1 AND favorites.article_id = $2",
        )
        .bind(user_id)
        .bind(article_id)
        .execute(&mut *conn)
        .await?;
        Ok(result.rows_affected() as usize)
    }

    pub async fn fetch_favorited_article_ids_by_username(
        conn: &mut PgConnection,
        username: &str,
    ) -> Result<Vec<Uuid>, AppError> {
        let ids = sqlx::query_scalar::<_, Uuid>(
            "SELECT favorites.article_id FROM favorites \
             INNER JOIN users ON favorites.user_id = users.id \
             WHERE users.username = $1",
        )
        .bind(username)
        .fetch_all(&mut *conn)
        .await?;
        Ok(ids)
    }
}

pub struct CreateFavorite {
    pub user_id: Uuid,
    pub article_id: Uuid,
}

pub struct DeleteFavorite {
    pub user_id: Uuid,
    pub article_id: Uuid,
}

#[derive(Clone)]
pub struct FavoriteInfo {
    pub is_favorited: bool,
    pub favorites_count: i64,
}
