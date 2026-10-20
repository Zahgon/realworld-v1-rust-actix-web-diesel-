use super::entities::Tag;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TagsResponse {
    // SPEC: https://gothinkster.github.io/realworld/docs/specs/backend-specs/endpoints#registration
    pub tags: Vec<String>,
}

impl std::convert::From<Vec<Tag>> for TagsResponse {
    fn from(tags: Vec<Tag>) -> Self {
        let list = tags.iter().map(move |tag| tag.name.clone()).collect();
        TagsResponse { tags: list }
    }
}

pub trait TagPresenter: Send + Sync + 'static {
    fn to_json(&self, list: Vec<Tag>) -> Response;
}

#[derive(Clone)]
pub struct TagPresenterImpl {}
impl TagPresenterImpl {
    pub fn new() -> Self {
        Self {}
    }
}
impl TagPresenter for TagPresenterImpl {
    fn to_json(&self, list: Vec<Tag>) -> Response {
        let res = TagsResponse::from(list);
        Json(res).into_response()
    }
}
