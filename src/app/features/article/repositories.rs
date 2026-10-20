use super::entities::{Article, CreateArticle, DeleteArticle, UpdateArticle};
use crate::app::features::favorite::entities::FavoriteInfo;
use crate::app::features::profile::entities::Profile;
use crate::app::features::tag::entities::{CreateTag, Tag};
use crate::app::features::user::entities::User;
use crate::error::AppError;
use crate::utils::db::DbPool;
use async_trait::async_trait;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

#[async_trait]
pub trait ArticleRepository: Send + Sync + 'static {
    async fn fetch_articles(
        &self,
        params: FetchArticlesRepositoryInput,
    ) -> Result<(ArticlesList, ArticlesCount), AppError>;

    async fn fetch_article_by_slug(
        &self,
        article_title_slug: String,
    ) -> Result<FetchArticleBySlugOutput, AppError>;

    async fn create_article(
        &self,
        params: CreateArticleRepositoryInput,
    ) -> Result<(Article, Profile, FavoriteInfo, Vec<Tag>), AppError>;

    async fn delete_article(&self, input: DeleteArticleRepositoryInput) -> Result<(), AppError>;

    async fn update_article(
        &self,
        input: UpdateArticleRepositoryInput,
    ) -> Result<(Article, Profile, FavoriteInfo, Vec<Tag>), AppError>;

    async fn fetch_article(
        &self,
        input: &FetchArticleRepositoryInput,
    ) -> Result<(Article, Profile, FavoriteInfo, Vec<Tag>), AppError>;

    async fn fetch_following_articles(
        &self,
        params: &FetchFollowingArticlesRepositoryInput,
    ) -> Result<(ArticlesList, ArticlesCount), AppError>;
}
#[derive(Clone)]
pub struct ArticleRepositoryImpl {
    pool: DbPool,
}

impl ArticleRepositoryImpl {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    async fn create_tag_list(
        conn: &mut PgConnection,
        tag_name_list: &Option<Vec<String>>,
        article_id: &Uuid,
    ) -> Result<Vec<Tag>, AppError> {
        match tag_name_list {
            Some(tag_name_list) => {
                let records = tag_name_list
                    .iter()
                    .map(|name| CreateTag { name, article_id })
                    .collect();
                Tag::create_list(conn, records).await
            }
            None => Ok(vec![]),
        }
    }

    /// The article-list filters, as `articles.id = ANY(..)` conditions with
    /// their id lists. A failed lookup aborts the request without a response.
    async fn article_filters(
        conn: &mut PgConnection,
        params: &FetchArticlesRepositoryInput,
    ) -> Vec<Vec<Uuid>> {
        use crate::app::features::favorite::entities::Favorite;
        let mut filters = Vec::new();

        if let Some(tag_name) = &params.tag {
            let ids = Tag::fetch_article_ids_by_name(conn, tag_name)
                .await
                .expect("could not fetch tagged article ids."); // TODO: us e ? or error handling
            filters.push(ids);
        }

        if let Some(author_name) = &params.author {
            let ids = Article::fetch_ids_by_author_name(conn, author_name)
                .await
                .expect("could not fetch authors id."); // TODO: use ? or error handling
            filters.push(ids);
        }

        if let Some(username) = &params.favorited {
            let ids = Favorite::fetch_favorited_article_ids_by_username(conn, username)
                .await
                .expect("could not fetch favorited articles id."); // TODO: use ? or error handling
            filters.push(ids);
        }

        filters
    }

    fn where_clause(filters: &[Vec<Uuid>]) -> String {
        if filters.is_empty() {
            return String::new();
        }
        let conditions = (1..=filters.len())
            .map(|i| format!("articles.id = ANY(${i})"))
            .collect::<Vec<_>>()
            .join(" AND ");
        format!(" WHERE {conditions}")
    }

    async fn favorite_info(
        conn: &mut PgConnection,
        article: &Article,
        user_id: &Uuid,
    ) -> Result<FavoriteInfo, AppError> {
        let is_favorited = article.is_favorited_by_user_id(conn, user_id).await?;
        let favorites_count = article.fetch_favorites_count(conn).await?;
        Ok(FavoriteInfo {
            is_favorited,
            favorites_count,
        })
    }

    async fn favorites_counts(
        conn: &mut PgConnection,
        article_and_user_list: &[(Article, User)],
    ) -> Result<Vec<i64>, AppError> {
        let mut list = Vec::with_capacity(article_and_user_list.len());
        for (article, _) in article_and_user_list {
            list.push(article.fetch_favorites_count(conn).await?);
        }
        Ok(list)
    }
}

#[async_trait]
impl ArticleRepository for ArticleRepositoryImpl {
    async fn fetch_articles(
        &self,
        params: FetchArticlesRepositoryInput,
    ) -> Result<(ArticlesList, ArticlesCount), AppError> {
        let mut conn = self.pool.acquire().await?;
        let conn = &mut *conn;

        let articles_count = {
            let filters = Self::article_filters(conn, &params).await;
            let sql = format!(
                "SELECT COUNT(articles.id) FROM articles \
                 INNER JOIN users ON articles.author_id = users.id{} LIMIT 1",
                Self::where_clause(&filters)
            );
            let mut query = sqlx::query_scalar::<_, i64>(&sql).persistent(false);
            for ids in &filters {
                query = query.bind(ids);
            }
            query.fetch_one(&mut *conn).await?
        };

        let result = {
            let article_and_user_list = {
                let filters = Self::article_filters(conn, &params).await;
                let sql = format!(
                    "SELECT {} FROM articles INNER JOIN users ON articles.author_id = users.id{} \
                     LIMIT ${} OFFSET ${}",
                    Article::with_author_columns(),
                    Self::where_clause(&filters),
                    filters.len() + 1,
                    filters.len() + 2
                );
                let mut query = sqlx::query(&sql).persistent(false);
                for ids in &filters {
                    query = query.bind(ids);
                }
                let rows = query
                    .bind(params.limit)
                    .bind(params.offset)
                    .fetch_all(&mut *conn)
                    .await?;
                rows.iter()
                    .map(Article::with_author_from_row)
                    .collect::<Result<Vec<_>, _>>()?
            };

            let tags_list = {
                let article_ids = article_and_user_list
                    .iter()
                    .map(|(article, _)| article.id)
                    .collect::<Vec<_>>();
                let tags_list = Tag::fetch_by_article_ids(conn, &article_ids, true).await?;
                Tag::grouped_by(tags_list, &article_ids)
            };

            let favorites_count_list = Self::favorites_counts(conn, &article_and_user_list).await?;

            article_and_user_list
                .into_iter()
                .zip(favorites_count_list)
                .map(|((article, user), favorites_count)| {
                    (
                        article,
                        Profile {
                            username: user.username,
                            bio: user.bio,
                            image: user.image,
                            following: false, // NOTE: because not authz
                        },
                        FavoriteInfo {
                            is_favorited: false, // NOTE: because not authz
                            favorites_count,
                        },
                    )
                })
                .zip(tags_list)
                .collect::<Vec<_>>()
        };

        Ok((result, articles_count))
    }

    async fn fetch_article_by_slug(
        &self,
        article_title_slug: String,
    ) -> Result<FetchArticleBySlugOutput, AppError> {
        let mut conn = self.pool.acquire().await?;
        let conn = &mut *conn;

        let (article, author) = Article::fetch_by_slug_with_author(conn, &article_title_slug).await?;

        let profile = author.fetch_profile(conn, &author.id).await?;

        let tags_list = Tag::fetch_by_article_id(conn, &article.id).await?;

        let favorite_info = Self::favorite_info(conn, &article, &author.id).await?;

        Ok((article, profile, favorite_info, tags_list))
    }

    async fn create_article(
        &self,
        params: CreateArticleRepositoryInput,
    ) -> Result<(Article, Profile, FavoriteInfo, Vec<Tag>), AppError> {
        let mut conn = self.pool.acquire().await?;
        let conn = &mut *conn;

        let article = Article::create(
            conn,
            &CreateArticle {
                author_id: params.current_user.id,
                slug: params.slug.clone(),
                title: params.title.clone(),
                description: params.description.clone(),
                body: params.body.clone(),
            },
        )
        .await?;

        let tag_list = Self::create_tag_list(conn, &params.tag_name_list, &article.id).await?;

        let profile = params
            .current_user
            .fetch_profile(conn, &article.author_id)
            .await?;

        let favorite_info = Self::favorite_info(conn, &article, &params.current_user.id).await?;

        Ok((article, profile, favorite_info, tag_list))
    }

    async fn delete_article(&self, input: DeleteArticleRepositoryInput) -> Result<(), AppError> {
        let mut conn = self.pool.acquire().await?;
        Article::delete(
            &mut conn,
            &DeleteArticle {
                slug: input.slug,
                author_id: input.author_id,
            },
        )
        .await
    }

    async fn update_article(
        &self,
        input: UpdateArticleRepositoryInput,
    ) -> Result<(Article, Profile, FavoriteInfo, Vec<Tag>), AppError> {
        let mut conn = self.pool.acquire().await?;
        let conn = &mut *conn;

        let article = Article::update(
            conn,
            &input.article_title_slug,
            &input.current_user.id,
            &UpdateArticle {
                slug: input.slug.to_owned(),
                title: input.title.to_owned(),
                description: input.description.to_owned(),
                body: input.body.to_owned(),
            },
        )
        .await?;

        let tag_list = Tag::fetch_by_article_id(conn, &article.id).await?;

        let profile = input
            .current_user
            .fetch_profile(conn, &article.author_id)
            .await?;

        let favorite_info = Self::favorite_info(conn, &article, &input.current_user.id).await?;

        Ok((article, profile, favorite_info, tag_list))
    }

    async fn fetch_article(
        &self,
        input: &FetchArticleRepositoryInput,
    ) -> Result<(Article, Profile, FavoriteInfo, Vec<Tag>), AppError> {
        let mut conn = self.pool.acquire().await?;
        let conn = &mut *conn;
        let (article, author) = Article::find_with_author(conn, &input.article_id).await?;

        let profile = input.current_user.fetch_profile(conn, &author.id).await?;

        let favorite_info = Self::favorite_info(conn, &article, &input.current_user.id).await?;

        let tags_list = Tag::fetch_by_article_id(conn, &article.id).await?;

        Ok((article, profile, favorite_info, tags_list))
    }

    async fn fetch_following_articles(
        &self,
        params: &FetchFollowingArticlesRepositoryInput,
    ) -> Result<(ArticlesList, ArticlesCount), AppError> {
        use crate::app::features::follow::entities::Follow;

        let mut conn = self.pool.acquire().await?;
        let conn = &mut *conn;

        let followee_ids =
            Follow::fetch_folowee_ids_by_follower_id(conn, &params.current_user.id).await?;

        let articles_list = {
            let article_and_user_list = {
                let rows = sqlx::query(&format!(
                    "SELECT {} FROM articles INNER JOIN users ON articles.author_id = users.id \
                     WHERE articles.author_id = ANY($1) \
                     ORDER BY articles.created_at DESC LIMIT $2 OFFSET $3",
                    Article::with_author_columns()
                ))
                .bind(&followee_ids)
                .bind(params.limit)
                .bind(params.offset)
                .fetch_all(&mut *conn)
                .await?;
                rows.iter()
                    .map(Article::with_author_from_row)
                    .collect::<Result<Vec<_>, _>>()?
            };

            let tags_list = {
                let article_ids = article_and_user_list
                    .iter()
                    .map(|(article, _)| article.id)
                    .collect::<Vec<_>>();
                let tags_list = Tag::fetch_by_article_ids(conn, &article_ids, false).await?;
                Tag::grouped_by(tags_list, &article_ids)
            };

            let follows_list = {
                let user_ids_list = article_and_user_list
                    .iter()
                    .map(|(_, user)| user.id)
                    .collect::<Vec<_>>();

                sqlx::query_as::<_, Follow>(&format!(
                    "SELECT {} FROM follows \
                     WHERE follows.follower_id = $1 AND follows.followee_id = ANY($2)",
                    Follow::COLUMNS
                ))
                .bind(params.current_user.id)
                .bind(&user_ids_list)
                .fetch_all(&mut *conn)
                .await?
            };

            let favorites_count_list = Self::favorites_counts(conn, &article_and_user_list).await?;

            let favorited_article_ids = params.current_user.fetch_favorited_article_ids(conn).await?;
            let is_favorited_by_me = |article: &Article| {
                favorited_article_ids
                    .iter()
                    .copied()
                    .any(|_id| _id == article.id)
            };

            article_and_user_list
                .into_iter()
                .zip(favorites_count_list)
                .map(|((article, user), favorites_count)| {
                    let following = follows_list.iter().any(|item| item.followee_id == user.id);
                    let is_favorited = is_favorited_by_me(&article);
                    (
                        article,
                        Profile {
                            username: user.username,
                            bio: user.bio,
                            image: user.image,
                            following,
                        },
                        FavoriteInfo {
                            is_favorited,
                            favorites_count,
                        },
                    )
                })
                .zip(tags_list)
                .collect::<Vec<_>>()
        };

        let articles_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(articles.id) FROM articles WHERE articles.author_id = ANY($1) LIMIT 1",
        )
        .bind(&followee_ids)
        .fetch_one(&mut *conn)
        .await?;

        Ok((articles_list, articles_count))
    }
}

pub struct CreateArticleRepositoryInput {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub body: String,
    pub tag_name_list: Option<Vec<String>>,
    pub current_user: User,
}

pub struct DeleteArticleRepositoryInput {
    pub slug: String,
    pub author_id: Uuid,
}

pub struct UpdateArticleRepositoryInput {
    pub current_user: User,
    pub article_title_slug: String,
    pub slug: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub body: Option<String>,
}

pub type FetchArticleBySlugOutput = (Article, Profile, FavoriteInfo, Vec<Tag>);

pub struct FetchArticlesRepositoryInput {
    pub tag: Option<String>,
    pub author: Option<String>,
    pub favorited: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct FetchArticleRepositoryInput {
    pub article_id: Uuid,
    pub current_user: User,
}

pub struct FetchFollowingArticlesRepositoryInput {
    pub current_user: User,
    pub offset: i64,
    pub limit: i64,
}

type ArticlesCount = i64;
type ArticlesListInner = (Article, Profile, FavoriteInfo);
pub type ArticlesList = Vec<(ArticlesListInner, Vec<Tag>)>;
