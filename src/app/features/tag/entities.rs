use crate::error::AppError;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgConnection, PgRow};
use sqlx::{FromRow, Row};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tag {
    pub id: Uuid,
    pub article_id: Uuid,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl<'r> FromRow<'r, PgRow> for Tag {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get(0)?,
            article_id: row.try_get(1)?,
            name: row.try_get(2)?,
            created_at: row.try_get(3)?,
            updated_at: row.try_get(4)?,
        })
    }
}

impl Tag {
    pub const COLUMNS: &'static str =
        "tags.id, tags.article_id, tags.name, tags.created_at, tags.updated_at";

    pub async fn fetch_by_article_id(
        conn: &mut PgConnection,
        article_id: &Uuid,
    ) -> Result<Vec<Self>, AppError> {
        let list = sqlx::query_as::<_, Self>(&format!(
            "SELECT {} FROM tags WHERE tags.article_id = $1",
            Self::COLUMNS
        ))
        .bind(article_id)
        .fetch_all(&mut *conn)
        .await?;
        Ok(list)
    }

    /// Tags of the given articles, optionally sorted by name.
    pub async fn fetch_by_article_ids(
        conn: &mut PgConnection,
        article_ids: &[Uuid],
        order_by_name: bool,
    ) -> Result<Vec<Self>, AppError> {
        let order = if order_by_name { " ORDER BY tags.name ASC" } else { "" };
        let list = sqlx::query_as::<_, Self>(&format!(
            "SELECT {} FROM tags WHERE tags.article_id = ANY($1){order}",
            Self::COLUMNS
        ))
        .bind(article_ids)
        .fetch_all(&mut *conn)
        .await?;
        Ok(list)
    }

    pub async fn fetch(conn: &mut PgConnection) -> Result<Vec<Self>, AppError> {
        let list = sqlx::query_as::<_, Self>(&format!("SELECT {} FROM tags", Self::COLUMNS))
            .fetch_all(&mut *conn)
            .await?;
        Ok(list)
    }

    pub async fn fetch_article_ids_by_name(
        conn: &mut PgConnection,
        tag_name: &str,
    ) -> Result<Vec<Uuid>, AppError> {
        let article_ids = sqlx::query_as::<_, Self>(&format!(
            "SELECT {} FROM tags WHERE tags.name = $1",
            Self::COLUMNS
        ))
        .bind(tag_name)
        .fetch_all(&mut *conn)
        .await?
        .iter()
        .map(|tag| tag.article_id)
        .collect();
        Ok(article_ids)
    }

    /// Inserts all records with one statement; an empty list inserts nothing.
    pub async fn create_list(
        conn: &mut PgConnection,
        records: Vec<CreateTag<'_>>,
    ) -> Result<Vec<Self>, AppError> {
        if records.is_empty() {
            return Ok(vec![]);
        }
        let values = (0..records.len())
            .map(|i| format!("(${}, ${})", 2 * i + 1, 2 * i + 2))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "INSERT INTO tags (name, article_id) VALUES {values} RETURNING {}",
            Self::COLUMNS
        );
        let mut query = sqlx::query_as::<_, Self>(&sql);
        for record in &records {
            query = query.bind(record.name).bind(record.article_id);
        }
        let tags_list = query.fetch_all(&mut *conn).await?;
        Ok(tags_list)
    }

    /// Splits `tags` into one list per article, in the order of `article_ids`.
    pub fn grouped_by(tags: Vec<Self>, article_ids: &[Uuid]) -> Vec<Vec<Self>> {
        let index: HashMap<Uuid, usize> = article_ids
            .iter()
            .enumerate()
            .map(|(i, id)| (*id, i))
            .collect();
        let mut grouped: Vec<Vec<Self>> = article_ids.iter().map(|_| Vec::new()).collect();
        for tag in tags {
            if let Some(&i) = index.get(&tag.article_id) {
                grouped[i].push(tag);
            }
        }
        grouped
    }
}

pub struct CreateTag<'a> {
    pub name: &'a str,
    pub article_id: &'a Uuid,
}
