#[macro_use]
extern crate log;

use tokio::net::TcpListener;
use tokio::signal::unix::{signal, SignalKind};
mod app;
mod constants;
mod error;
mod utils;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    println!("start conduit server...");
    std::env::set_var("RUST_LOG", "conduit=warn");
    env_logger::init();

    let state = {
        let pool = utils::db::establish_connection().await;
        use app::drivers::middlewares::state::AppState;
        AppState::new(pool)
    };

    let app = app::drivers::routes::api(state);
    let listener = TcpListener::bind(constants::BIND).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
}

/// Resolves on SIGINT or SIGTERM; the server then stops accepting connections
/// and lets in-flight requests finish.
async fn shutdown_signal() {
    let mut interrupt = signal(SignalKind::interrupt()).expect("cannot install SIGINT handler");
    let mut terminate = signal(SignalKind::terminate()).expect("cannot install SIGTERM handler");
    tokio::select! {
        _ = interrupt.recv() => {}
        _ = terminate.recv() => {}
    }
}
