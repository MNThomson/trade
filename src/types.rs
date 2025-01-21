use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use fake::{Dummy, faker::company::en::CompanyName};
use serde::{Deserialize, Serialize};
use serde_json::{self, json};
use ulid::Ulid;

use crate::DB;

#[derive(Clone)]
pub struct AppState {
    pub db: DB,
}

#[derive(Serialize, Deserialize, Debug, Dummy)]
pub struct StockPrice {
    pub stock_id: Ulid,
    #[dummy(faker = "CompanyName()")]
    pub stock_name: String,
    #[dummy(faker = "1..200")]
    pub current_price: usize,
}

#[derive(Serialize, Deserialize, Debug, Dummy)]
pub struct StockPortfolio {
    pub stock_id: Ulid,
    #[dummy(faker = "CompanyName()")]
    pub stock_name: String,
    #[dummy(faker = "1..1000")]
    pub quantity_owned: usize,
}

#[derive(Serialize, Deserialize, Debug, Dummy)]
pub struct WalletTransaction {
    pub wallet_tx_id: Ulid,
    pub stock_tx_id: Ulid,
    pub is_debit: bool,
    #[dummy(faker = "1..10000")]
    pub amount: usize,
    pub time_stamp: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Dummy)]
pub enum OrderStatus {
    PENDING,
    COMPLETED,
    CANCELLED,
}

#[derive(Serialize, Deserialize, Debug, Dummy)]
pub enum OrderType {
    MARKET,
    LIMIT,
}

#[derive(Serialize, Deserialize, Debug, Dummy)]
pub struct StockTransaction {
    pub stock_tx_id: Ulid,
    pub stock_id: Ulid,
    pub wallet_tx_id: Ulid,
    pub order_status: OrderStatus,
    pub is_buy: bool,
    pub order_type: OrderType,
    #[dummy(faker = "1..200")]
    pub stock_price: usize,
    #[dummy(faker = "1..200")]
    pub quantity: usize,
    pub time_stamp: DateTime<Utc>,
}

#[derive(Debug)]
pub enum ApiResponse {
    /// No response body
    None,
    /// No response body with HTTP 201
    NoneCreated,
    /// Custom error message
    Error(String),
    /// JWT token
    Token(String),
    /// List of stock prices
    StockPriceVec(Vec<StockPrice>),
    /// List of stocks owned
    StockPortfolioVec(Vec<StockPortfolio>),
    /// Account balance
    Balance(i64),
    /// Account withdrawls/deposits
    WalletVec(Vec<WalletTransaction>),
    /// Stock trades
    TradeVec(Vec<StockTransaction>),
    /// StockID after creation
    StockId(String),
}

fn success<T: Serialize>(input: &T) -> String {
    json!({
        "success": true,
        "data": input,
    })
    .to_string()
}

impl IntoResponse for ApiResponse {
    #[tracing::instrument]
    fn into_response(self) -> Response {
        match self {
            ApiResponse::None => (StatusCode::OK, success(&Some(()))).into_response(),
            ApiResponse::NoneCreated => (StatusCode::CREATED, success(&Some(()))).into_response(),
            ApiResponse::Error(e) => (
                StatusCode::BAD_REQUEST,
                json!({ "success": false, "data": {"error": e} }).to_string(),
            )
                .into_response(),
            ApiResponse::Token(t) => {
                (StatusCode::OK, success(&json!({"token": t}))).into_response()
            }
            ApiResponse::StockPriceVec(s) => (StatusCode::OK, success(&s)).into_response(),
            ApiResponse::StockPortfolioVec(s) => (StatusCode::OK, success(&s)).into_response(),
            ApiResponse::Balance(b) => {
                (StatusCode::OK, success(&json!({"balance": b}))).into_response()
            }
            ApiResponse::WalletVec(w) => (StatusCode::OK, success(&w)).into_response(),
            ApiResponse::TradeVec(t) => (StatusCode::OK, success(&t)).into_response(),
            ApiResponse::StockId(id) => {
                (StatusCode::OK, success(&json!({"stock_id": id}))).into_response()
            }
        }
    }
}
