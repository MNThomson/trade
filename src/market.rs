use axum::extract::State;

use crate::{
    AppState,
    auth::AuthUser,
    types::{AppError, Balance, StockPortfolioVec, StockPriceVec, TradeVec, WalletVec},
};

#[tracing::instrument(skip_all)]
pub async fn get_stock_prices(
    AuthUser(_user): AuthUser,
    State(state): State<AppState>,
) -> Result<StockPriceVec, AppError> {
    let out = state.db.get_stock_prices().await?;
    Ok(StockPriceVec(out))
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
pub async fn get_wallet_balance(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> Result<Balance, AppError> {
    let bal = state.db.get_wallet_balance(user).await?;
    Ok(Balance { balance: bal })
}

#[tracing::instrument(skip_all)]
pub async fn get_wallet_transactions(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> Result<WalletVec, AppError> {
    let out = state.db.get_wallet_transactions(user).await?;
    Ok(WalletVec(out))
}

#[tracing::instrument(skip_all)]
pub async fn get_stock_transactions(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> Result<TradeVec, AppError> {
    let out = state.db.get_stock_transactions(user).await?;
    Ok(TradeVec(out))
}
