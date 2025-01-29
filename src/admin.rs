use axum::{Json, extract::State};
use serde::Deserialize;

use crate::{
    AppState,
    auth::AuthUser,
    types::{AppError, EmptyCreatedResponse, EmptyResponse, StockId},
};

#[derive(Deserialize)]
pub struct AddMoneyRequest {
    amount: i64,
}

#[tracing::instrument(skip_all)]
pub async fn add_money_to_wallet(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(body): Json<AddMoneyRequest>,
) -> Result<EmptyCreatedResponse, AppError> {
    state.db.add_money_to_user(user, body.amount).await?;
    Ok(EmptyCreatedResponse {})
}

#[derive(Deserialize)]
pub struct AddStockToUserRequest {
    stock_id: String,
    quantity: i64,
}

#[tracing::instrument(skip_all)]
pub async fn add_stock_to_user(
    State(_state): State<AppState>,
    Json(_payload): Json<AddStockToUserRequest>,
) -> EmptyResponse {
    EmptyResponse {}
}

#[derive(Deserialize)]
pub struct CreateStockRequest {
    stock_name: String,
}

#[tracing::instrument(skip_all)]
pub async fn create_stock(
    State(_state): State<AppState>,
    Json(_payload): Json<CreateStockRequest>,
) -> StockId {
    StockId {
        stock_id: "stock_id".to_string(),
    }
}
