use crate::app::features::user::entities::User;
use crate::error::AppError;
use crate::utils::converter;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgConnection, PgRow};
use sqlx::{FromRow, Row};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Article {
    pub id: Uuid,
    pub author_id: Uuid,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub body: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl Article {
    pub const COLUMNS: &'static str = "articles.id, articles.author_id, articles.slug, \
        articles.title, articles.description, articles.body, articles.created_at, \
        articles.updated_at";

    /// Index of the first author column in rows selected with `WITH_AUTHOR_COLUMNS`.
    pub const AUTHOR_OFFSET: usize = 8;

    pub fn from_row_at(row: &PgRow, start: usize) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get(start)?,
            author_id: row.try_get(start + 1)?,
            slug: row.try_get(start + 2)?,
            title: row.try_get(start + 3)?,
            description: row.try_get(start + 4)?,
            body: row.try_get(start + 5)?,
            created_at: row.try_get(start + 6)?,
            updated_at: row.try_get(start + 7)?,
        })
    }

    pub fn with_author_from_row(row: &PgRow) -> Result<(Self, User), sqlx::Error> {
        Ok((
            Self::from_row_at(row, 0)?,
            User::from_row_at(row, Self::AUTHOR_OFFSET)?,
        ))
    }

    pub fn with_author_columns() -> String {
        format!("{}, {}", Self::COLUMNS, User::COLUMNS)
    }
}

impl<'r> FromRow<'r, PgRow> for Article {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Self::from_row_at(row, 0)
    }
}

impl Article {
    pub async fn create(conn: &mut PgConnection, record: &CreateArticle) -> Result<Self, AppError> {
        let article = sqlx::query_as::<_, Article>(&format!(
            "INSERT INTO articles (author_id, slug, title, description, body) \
             VALUES ($1, $2, $3, $4, $5) RETURNING {}",
            Self::COLUMNS
        ))
        .bind(record.author_id)
        .bind(&record.slug)
        .bind(&record.title)
        .bind(&record.description)
        .bind(&record.body)
        .fetch_one(&mut *conn)
        .await?;

        Ok(article)
    }

    pub async fn update(
        conn: &mut PgConnection,
        article_title_slug: &str,
        author_id: &Uuid,
        record: &UpdateArticle,
    ) -> Result<Self, AppError> {
        // Only the fields present in the record are written; with none
        // present there is nothing to save, which the API answers with a 500.
        let fields = [
            ("slug", &record.slug),
            ("title", &record.title),
            ("description", &record.description),
            ("body", &record.body),
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
            "UPDATE articles SET {} WHERE articles.slug = ${} AND articles.author_id = ${} \
             RETURNING {}",
            assignments.join(", "),
            values.len() + 1,
            values.len() + 2,
            Self::COLUMNS
        );
        let mut query = sqlx::query_as::<_, Article>(&sql);
        for value in values {
            query = query.bind(value);
        }
        let article = query
            .bind(article_title_slug)
            .bind(author_id)
            .fetch_one(&mut *conn)
            .await?;
        Ok(article)
    }

    pub fn convert_title_to_slug(title: &str) -> String {
        converter::to_kebab(title)
    }

    pub async fn fetch_by_slug_and_author_id(
        conn: &mut PgConnection,
        params: &FetchBySlugAndAuthorId,
    ) -> Result<Self, AppError> {
        let item = sqlx::query_as::<_, Article>(&format!(
            "SELECT {} FROM articles WHERE articles.slug = $1 AND articles.author_id = $2 LIMIT 1",
            Self::COLUMNS
        ))
        .bind(&params.slug)
        .bind(params.author_id)
        .fetch_one(&mut *conn)
        .await?;
        Ok(item)
    }

    pub async fn fetch_by_slug_with_author(
        conn: &mut PgConnection,
        slug: &str,
    ) -> Result<(Self, User), AppError> {
        let row = sqlx::query(&format!(
            "SELECT {} FROM articles INNER JOIN users ON articles.author_id = users.id \
             WHERE articles.slug = $1",
            Self::with_author_columns()
        ))
        .bind(slug)
        .fetch_one(&mut *conn)
        .await?;
        Ok(Self::with_author_from_row(&row)?)
    }

    pub async fn fetch_ids_by_author_name(
        conn: &mut PgConnection,
        name: &str,
    ) -> Result<Vec<Uuid>, AppError> {
        let ids = sqlx::query_scalar::<_, Uuid>(
            "SELECT articles.id FROM users INNER JOIN articles ON articles.author_id = users.id \
             WHERE users.username = $1",
        )
        .bind(name)
        .fetch_all(&mut *conn)
        .await?;
        Ok(ids)
    }

    pub async fn find_with_author(
        conn: &mut PgConnection,
        id: &Uuid,
    ) -> Result<(Self, User), AppError> {
        let row = sqlx::query(&format!(
            "SELECT {} FROM articles INNER JOIN users ON articles.author_id = users.id \
             WHERE articles.id = $1",
            Self::with_author_columns()
        ))
        .bind(id)
        .fetch_one(&mut *conn)
        .await?;
        Ok(Self::with_author_from_row(&row)?)
    }

    pub async fn delete(conn: &mut PgConnection, params: &DeleteArticle) -> Result<(), AppError> {
        sqlx::query("DELETE FROM articles WHERE articles.slug = $1 AND articles.author_id = $2")
            .bind(&params.slug)
            .bind(params.author_id)
            .execute(&mut *conn)
            .await?;
        // NOTE: references tag rows are deleted automatically by DELETE CASCADE

        Ok(())
    }
}

impl Article {
    pub async fn is_favorited_by_user_id(
        &self,
        conn: &mut PgConnection,
        user_id: &Uuid,
    ) -> Result<bool, AppError> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(favorites.id) FROM favorites \
             WHERE favorites.article_id = $1 AND favorites.user_id = $2 LIMIT 1",
        )
        .bind(self.id)
        .bind(user_id)
        .fetch_one(&mut *conn)
        .await?;
        Ok(count >= 1)
    }

    pub async fn fetch_favorites_count(&self, conn: &mut PgConnection) -> Result<i64, AppError> {
        let favorites_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(favorites.created_at) FROM favorites \
             WHERE favorites.article_id = $1 LIMIT 1",
        )
        .bind(self.id)
        .fetch_one(&mut *conn)
        .await?;
        Ok(favorites_count)
    }
}

#[derive(Clone)]
pub struct CreateArticle {
    pub author_id: Uuid,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub body: String,
}

pub struct UpdateArticle {
    pub slug: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub body: Option<String>,
}

pub struct FetchBySlugAndAuthorId {
    pub slug: String,
    pub author_id: Uuid,
}

pub struct DeleteArticle {
    pub slug: String,
    pub author_id: Uuid,
}
