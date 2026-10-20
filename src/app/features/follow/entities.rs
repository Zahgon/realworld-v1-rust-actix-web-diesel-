use crate::error::AppError;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgConnection, PgRow};
use sqlx::{FromRow, Row};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
pub struct Follow {
    pub followee_id: Uuid,
    pub follower_id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl<'r> FromRow<'r, PgRow> for Follow {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            followee_id: row.try_get(0)?,
            follower_id: row.try_get(1)?,
            created_at: row.try_get(2)?,
            updated_at: row.try_get(3)?,
        })
    }
}

impl Follow {
    pub const COLUMNS: &'static str =
        "follows.followee_id, follows.follower_id, follows.created_at, follows.updated_at";

    pub async fn create(conn: &mut PgConnection, params: &CreateFollow) -> Result<(), AppError> {
        sqlx::query("INSERT INTO follows (follower_id, followee_id) VALUES ($1, $2)")
            .bind(params.follower_id)
            .bind(params.followee_id)
            .execute(&mut *conn)
            .await?;
        Ok(())
    }

    pub async fn delete(conn: &mut PgConnection, params: &DeleteFollow) -> Result<(), AppError> {
        sqlx::query("DELETE FROM follows WHERE follows.followee_id = $1 AND follows.follower_id = $2")
            .bind(params.followee_id)
            .bind(params.follower_id)
            .execute(&mut *conn)
            .await?;
        Ok(())
    }

    pub async fn fetch_folowee_ids_by_follower_id(
        conn: &mut PgConnection,
        follower_id: &Uuid,
    ) -> Result<Vec<Uuid>, AppError> {
        let result = sqlx::query_scalar::<_, Uuid>(
            "SELECT follows.followee_id FROM follows WHERE follows.follower_id = $1",
        )
        .bind(follower_id)
        .fetch_all(&mut *conn)
        .await?;
        Ok(result)
    }
}

pub struct CreateFollow {
    pub follower_id: Uuid,
    pub followee_id: Uuid,
}

pub struct DeleteFollow {
    pub follower_id: Uuid,
    pub followee_id: Uuid,
}
