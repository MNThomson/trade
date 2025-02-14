use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    auth::AuthUser,
    types::{AppError, EmptyCreatedResponse, EmptyResponse, StockId},
};

#[derive(Deserialize, Serialize)]
pub struct AddMoneyRequest {
    pub amount: i64,
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

#[derive(Serialize, Deserialize)]
pub struct AddStockToUserRequest {
    pub stock_id: String,
    pub quantity: i64,
}

#[tracing::instrument(skip_all)]
pub async fn add_stock_to_user(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(body): Json<AddStockToUserRequest>,
) -> Result<EmptyResponse, AppError> {
    let stock_id = body.stock_id.parse().map_err(|_| AppError::StockNotFound)?;
    state
        .db
        .add_stock_to_user(user, stock_id, body.quantity)
        .await?;
    Ok(EmptyResponse {})
}

#[derive(Serialize, Deserialize)]
pub struct CreateStockRequest {
    pub stock_name: String,
}

#[tracing::instrument(skip_all)]
pub async fn create_stock(
    AuthUser(_user): AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CreateStockRequest>,
) -> Result<StockId, AppError> {
    let stock_id = state.db.create_stock(body.stock_name).await?.to_string();

    Ok(StockId { stock_id })
}
