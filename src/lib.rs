#![feature(assert_matches)]
use std::any::Any;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Response, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use hypertext::*;
use tower_http::catch_panic::CatchPanicLayer;
use tracing::error;

use crate::{auth::AuthUser, db::DB, frontend::home, telemetry::otel_tracing, types::AppState};

pub mod admin;
pub mod auth;
pub mod db;
pub mod frontend;
pub mod hypertxt;
#[cfg(test)]
pub mod integration;
pub mod market;
pub mod order;
pub mod telemetry;
pub mod types;
pub mod user;

pub async fn router(state: AppState) -> Router {
    Router::new()
        // Frontend
        .route("/", get(home().render()))
        // User
        .route("/authentication/login", post(user::login))
        .route("/authentication/register", post(user::register))
        // Market
        .route("/transaction/getStockPrices", get(market::get_stock_prices))
        .route(
            "/transaction/getStockPortfolio",
            get(market::get_stock_portfolio),
        )
        .route(
            "/transaction/getWalletBalance",
            get(market::get_wallet_balance),
        )
        .route(
            "/transaction/getWalletTransactions",
            get(market::get_wallet_transactions),
        )
        .route(
            "/transaction/getStockTransactions",
            get(market::get_stock_transactions),
        )
        // Order
        .route("/engine/placeStockOrder", post(order::place_stock_order))
        .route(
            "/engine/cancelStockTransaction",
            post(order::cancel_stock_transaction),
        )
        // Admin
        .route(
            "/transaction/addMoneyToWallet",
            post(admin::add_money_to_wallet),
        )
        .route("/setup/addStockToUser", post(admin::add_stock_to_user))
        .route("/setup/createStock", post(admin::create_stock))
        // Misc
        .route("/protected", get(protected))
        .layer(otel_tracing())
        .route("/health", get(healthcheck))
        .with_state(state)
        .route("/version", get(|| async { env!("GIT_HASH") }))
        .layer(CatchPanicLayer::custom(handle_panic))
}

#[tracing::instrument(skip(user))]
async fn protected(AuthUser(user): AuthUser) -> impl IntoResponse {
    user.to_string()
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
