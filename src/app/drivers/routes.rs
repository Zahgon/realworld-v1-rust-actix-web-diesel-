use crate::app;
use crate::app::drivers::middlewares::{auth, cors, state::AppState};
use axum::extract::{RawPathParams, Request};
use axum::http::{header, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, MethodRouter};
use axum::Router;

pub fn api(state: AppState) -> Router {
    use app::features::{article, comment, favorite, healthcheck, profile, tag, user};

    let routes = Router::new()
        .route(
            "/api/healthcheck",
            only(get(healthcheck::controllers::index)),
        )
        .route("/api/tags", only(get(tag::controllers::index)))
        .route("/api/users/login", only(post(user::controllers::signin)))
        .route("/api/users", only(post(user::controllers::signup)))
        .route(
            "/api/user",
            only(get(user::controllers::me).put(user::controllers::update)),
        )
        .route(
            "/api/profiles/:username",
            only(get(profile::controllers::show)),
        )
        .route(
            "/api/profiles/:username/follow",
            only(post(profile::controllers::follow).delete(profile::controllers::unfollow)),
        )
        .route("/api/articles/feed", only(get(article::controllers::feed)))
        .route(
            "/api/articles",
            only(get(article::controllers::index).post(article::controllers::create)),
        )
        .route(
            "/api/articles/:article_title_slug",
            only(
                get(article::controllers::show)
                    .put(article::controllers::update)
                    .delete(article::controllers::delete),
            ),
        )
        .route(
            "/api/articles/:article_title_slug/favorite",
            only(post(favorite::controllers::favorite).delete(favorite::controllers::unfavorite)),
        )
        .route(
            "/api/articles/:article_title_slug/comments",
            only(get(comment::controllers::index).post(comment::controllers::create)),
        )
        .route(
            "/api/articles/:article_title_slug/comments/:comment_id",
            only(delete(comment::controllers::delete)),
        )
        .route_layer(middleware::from_fn(non_empty_params))
        .fallback(not_found)
        .with_state(state.clone());

    // Layers on a router whose only service is its fallback wrap the whole
    // route table: authentication and CORS see every request before routing.
    Router::new()
        .fallback_service(routes)
        .layer(middleware::map_response(without_allow_header))
        .layer(middleware::from_fn_with_state(cors::cors(), cors::handle))
        .layer(middleware::from_fn_with_state(state, auth::authentication))
}

/// Routes match on path and method together: a method with no handler on a
/// path, HEAD included (axum would otherwise answer it with the GET handler),
/// gets the same empty 404 as an unknown path.
fn only(router: MethodRouter<AppState>) -> MethodRouter<AppState> {
    router.head(not_found).fallback(not_found)
}

/// A path parameter matches one or more characters, so a path with an empty
/// segment in a parameter's place matches no route.
async fn non_empty_params(params: Option<RawPathParams>, req: Request, next: Next) -> Response {
    let has_empty_param = params
        .as_ref()
        .is_some_and(|params| params.iter().any(|(_, value)| value.is_empty()));
    if has_empty_param {
        return not_found().await.into_response();
    }
    next.run(req).await
}

/// Responses never list a path's methods, including on the 404 for a method
/// the path does not serve.
async fn without_allow_header(mut res: Response) -> Response {
    res.headers_mut().remove(header::ALLOW);
    res
}

async fn not_found() -> StatusCode {
    StatusCode::NOT_FOUND
}
