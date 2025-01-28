use axum::serve;
use tracing::info;
use trade::{db::DB, router, telemetry::tracing_init, types::AppState};

#[tokio::main]
async fn main() {
    tracing_init(env!("CARGO_PKG_NAME"), env!("GIT_HASH"));

    let state = AppState {
        db: DB::init().await.unwrap(),
    };

    let app = router(state).await;

    #[cfg(debug_assertions)]
    let app = app.layer(tower_livereload::LiveReloadLayer::new().request_predicate(
        |req: &axum::http::Request<_>| !req.headers().contains_key("hx-request"),
    ));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    info!("listening on {}", listener.local_addr().unwrap());
    serve(listener, app).await.unwrap();
}
