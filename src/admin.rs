use axum::{Json, extract::State};
use serde::Deserialize;

use crate::{AppState, types::ApiResponse};

#[derive(Deserialize)]
pub struct AddMoneyRequest {
    amount: i64,
}

#[tracing::instrument(skip_all)]
pub async fn add_money_to_wallet(
    State(_state): State<AppState>,
    Json(_payload): Json<AddMoneyRequest>,
) -> ApiResponse {
    ApiResponse::None
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
) -> ApiResponse {
    ApiResponse::None
}

#[derive(Deserialize)]
pub struct CreateStockRequest {
    stock_name: String,
}

#[tracing::instrument(skip_all)]
pub async fn create_stock(
    State(_state): State<AppState>,
    Json(_payload): Json<CreateStockRequest>,
) -> ApiResponse {
    ApiResponse::StockId("your_stock_id".to_string())
}
