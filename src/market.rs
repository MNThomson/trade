use axum::extract::State;
use fake::{Fake, Faker};

use crate::{AppState, types::ApiResponse};

#[tracing::instrument(skip_all)]
pub async fn get_stock_prices(State(_state): State<AppState>) -> ApiResponse {
    ApiResponse::StockPriceVec(vec![Faker.fake(), Faker.fake()])
}

#[tracing::instrument(skip_all)]
pub async fn get_stock_portfolio(State(_state): State<AppState>) -> ApiResponse {
    ApiResponse::StockPortfolioVec(vec![Faker.fake(), Faker.fake()])
}

#[tracing::instrument(skip_all)]
pub async fn get_wallet_balance(State(_state): State<AppState>) -> ApiResponse {
    ApiResponse::Balance(100)
}

#[tracing::instrument(skip_all)]
pub async fn get_wallet_transactions(State(_state): State<AppState>) -> ApiResponse {
    ApiResponse::WalletVec(vec![Faker.fake(), Faker.fake()])
}

#[tracing::instrument(skip_all)]
pub async fn get_stock_transactions(State(_state): State<AppState>) -> ApiResponse {
    ApiResponse::TradeVec(vec![Faker.fake(), Faker.fake()])
}
