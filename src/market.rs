use axum::extract::State;
use fake::{Fake, Faker};

use crate::{
    AppState,
    auth::AuthUser,
    types::{AppError, Balance, StockPortfolioVec, StockPriceVec, TradeVec, WalletVec},
};

#[tracing::instrument(skip_all)]
pub async fn get_stock_prices(State(_state): State<AppState>) -> StockPriceVec {
    StockPriceVec(vec![Faker.fake(), Faker.fake()])
}

#[tracing::instrument(skip_all)]
pub async fn get_stock_portfolio(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> Result<StockPortfolioVec, AppError> {
    let out = state.db.get_stock_portfolio(user).await?;
    Ok(StockPortfolioVec(out))
}

#[tracing::instrument(skip_all)]
pub async fn get_wallet_balance(State(_state): State<AppState>) -> Balance {
    Balance { balance: 100 }
}

#[tracing::instrument(skip_all)]
pub async fn get_wallet_transactions(State(_state): State<AppState>) -> WalletVec {
    WalletVec(vec![Faker.fake(), Faker.fake()])
}

#[tracing::instrument(skip_all)]
pub async fn get_stock_transactions(State(_state): State<AppState>) -> TradeVec {
    TradeVec(vec![Faker.fake(), Faker.fake()])
}
