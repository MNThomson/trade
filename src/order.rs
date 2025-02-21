use axum::extract::{Json, State};
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthUser,
    types::{AppError, AppState, EmptyCreatedResponse, EmptyResponse, OrderType},
};

#[derive(Serialize, Deserialize)]
pub struct PlaceStockOrderRequest {
    pub stock_id: String,
    pub is_buy: bool,
    pub order_type: OrderType,
    pub quantity: i64,
    pub price: Option<i64>,
}

#[tracing::instrument(skip_all)]
pub async fn place_stock_order(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(body): Json<PlaceStockOrderRequest>,
) -> Result<EmptyCreatedResponse, AppError> {
    if !(body.is_buy && matches!(body.order_type, OrderType::Market) && body.price.is_none()
        || (!body.is_buy && matches!(body.order_type, OrderType::Limit) && body.price.is_some()))
    {
        return Err(AppError::BadRequest);
    }
    if !body.is_buy {
        state
            .db
            .create_sell_order(
                user,
                body.stock_id.parse().map_err(|_| AppError::StockNotFound)?,
                body.quantity,
                body.price.expect("is a sell order"),
            )
            .await?;
        return Ok(EmptyCreatedResponse {});
    }

    state
        .db
        .create_buy_order(
            user,
            body.stock_id.parse().map_err(|_| AppError::StockNotFound)?,
            body.quantity,
        )
        .await?;

    Ok(EmptyCreatedResponse {})
}

#[allow(unused)]
#[derive(Deserialize, Serialize)]
pub struct CancelStockTransactionRequest {
    pub stock_tx_id: String,
}

#[tracing::instrument(skip_all)]
pub async fn cancel_stock_transaction(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CancelStockTransactionRequest>,
) -> Result<EmptyResponse, AppError> {
    state
        .db
        .cancel_sell_order(
            user,
            body.stock_tx_id
                .parse()
                .map_err(|_| AppError::StockTransactionNotFound)?,
        )
        .await?;
    Ok(EmptyResponse {})
}
