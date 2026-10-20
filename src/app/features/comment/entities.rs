use crate::app::features::user::entities::User;
use crate::error::AppError;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgConnection, PgRow};
use sqlx::{FromRow, Row};
use uuid::Uuid;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Comment {
    pub id: Uuid,
    pub article_id: Uuid,
    pub author_id: Uuid,
    pub body: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl Comment {
    // The table's creation-time column is named `create_at`.
    pub const COLUMNS: &'static str = "comments.id, comments.article_id, comments.author_id, \
        comments.body, comments.create_at, comments.updated_at";

    /// Index of the first author column in rows selected with `with_author_columns`.
    pub const AUTHOR_OFFSET: usize = 6;

    pub fn from_row_at(row: &PgRow, start: usize) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get(start)?,
            article_id: row.try_get(start + 1)?,
            author_id: row.try_get(start + 2)?,
            body: row.try_get(start + 3)?,
            created_at: row.try_get(start + 4)?,
            updated_at: row.try_get(start + 5)?,
        })
    }

    pub fn with_author_columns() -> String {
        format!("{}, {}", Self::COLUMNS, User::COLUMNS)
    }

    pub fn with_author_from_row(row: &PgRow) -> Result<(Self, User), sqlx::Error> {
        Ok((
            Self::from_row_at(row, 0)?,
            User::from_row_at(row, Self::AUTHOR_OFFSET)?,
        ))
    }
}

impl<'r> FromRow<'r, PgRow> for Comment {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Self::from_row_at(row, 0)
    }
}

impl Comment {
    pub async fn create(conn: &mut PgConnection, record: &CreateComment) -> Result<Self, AppError> {
        let new_comment = sqlx::query_as::<_, Comment>(&format!(
            "INSERT INTO comments (body, author_id, article_id) VALUES ($1, $2, $3) RETURNING {}",
            Self::COLUMNS
        ))
        .bind(&record.body)
        .bind(record.author_id)
        .bind(record.article_id)
        .fetch_one(&mut *conn)
        .await?;
        Ok(new_comment)
    }

    pub async fn delete(
        conn: &mut PgConnection,
        (comment_id, author_id, slug): (&Uuid, &Uuid, &str),
    ) -> Result<(), AppError> {
        sqlx::query(
            "DELETE FROM comments WHERE comments.id = $1 AND comments.author_id = $2 \
             AND comments.article_id IN (SELECT articles.id FROM articles \
             WHERE articles.slug = $3 AND articles.author_id = $4)",
        )
        .bind(comment_id)
        .bind(author_id)
        .bind(slug)
        .bind(author_id)
        .execute(&mut *conn)
        .await?;
        Ok(())
    }

    pub async fn fetch_all_with_author(
        conn: &mut PgConnection,
    ) -> Result<Vec<(Self, User)>, AppError> {
        let rows = sqlx::query(&format!(
            "SELECT {} FROM comments INNER JOIN users ON comments.author_id = users.id",
            Self::with_author_columns()
        ))
        .fetch_all(&mut *conn)
        .await?;
        let list = rows
            .iter()
            .map(Self::with_author_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(list)
    }
}

#[derive(Clone)]
pub struct CreateComment {
    pub body: String,
    pub author_id: Uuid,
    pub article_id: Uuid,
}
