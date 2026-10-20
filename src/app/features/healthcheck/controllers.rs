use axum::body::Body;
use axum::response::Response;

pub async fn index() -> Response {
    Response::new(Body::from("OK"))
}
