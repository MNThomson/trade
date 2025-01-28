use axum::extract::Json;
use serde::Deserialize;

use crate::types::{ApiResponse, OrderType};

#[derive(Deserialize)]
pub struct PlaceStockOrderRequest {
    stock_id: String,
    is_buy: bool,
    order_type: OrderType,
    quantity: u32,
    price: Option<u32>,
}

#[tracing::instrument(skip_all)]
pub async fn place_stock_order(Json(_payload): Json<PlaceStockOrderRequest>) -> ApiResponse {
    ApiResponse::None
}

#[derive(Deserialize)]
pub struct CancelStockTransactionRequest {
    stock_tx_id: String,
}
#[tracing::instrument(skip_all)]
pub async fn cancel_stock_transaction(
    Json(_payload): Json<CancelStockTransactionRequest>,
) -> ApiResponse {
    ApiResponse::None
}
