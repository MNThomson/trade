use std::any::Any;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Response, StatusCode},
    response::IntoResponse,
    routing::get,
};
use db::DB;
use frontend::home;
use hypertext::*;
use telemetry::{otel_tracing, tracing_init};
use tower_http::catch_panic::CatchPanicLayer;
use tracing::{error, info};

use crate::auth::AuthUser;

mod auth;
mod frontend;

#[derive(Clone)]
struct AppState {
    db: DB,
}

#[tokio::main]
async fn main() {
    tracing_init(env!("CARGO_PKG_NAME"), env!("GIT_HASH"));

    let state = AppState {
        db: DB::init().await.unwrap(),
    };

    let app = Router::new()
        .route("/", get(home().render()))
        .route("/protected", get(protected))
        .layer(otel_tracing())
        .route("/health", get(healthcheck))
        .with_state(state)
        .route("/version", get(|| async { env!("GIT_HASH") }))
        .layer(CatchPanicLayer::custom(handle_panic));

    #[cfg(debug_assertions)]
    let app = app.layer(tower_livereload::LiveReloadLayer::new().request_predicate(
        |req: &axum::http::Request<_>| !req.headers().contains_key("hx-request"),
    ));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

#[tracing::instrument(skip(user))]
async fn protected(AuthUser(user): AuthUser) -> impl IntoResponse {
    user.session_token
}

async fn healthcheck(State(state): State<AppState>) -> impl IntoResponse {
    let is_healthy = state.db.healthcheck().await.is_ok();
    let health_status = if is_healthy { "healthy" } else { "unhealthy" };
    let statuscode = if is_healthy {
        StatusCode::OK
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    };

    (statuscode, rsx!(<p>database: {health_status}</p>).render())
}

fn handle_panic(err: Box<dyn Any + Send + 'static>) -> Response<Body> {
    let details = if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = err.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "Unknown panic message".to_string()
    };
    error!(details);

    (StatusCode::INTERNAL_SERVER_ERROR).into_response()
}
