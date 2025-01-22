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
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    Cancelled = -1,
    Completed = 0,
    Pending = 1,
    InProgress = 2,
}

#[derive(Serialize, Deserialize, Debug, Dummy)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderType {
    Market,
    Limit,
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
    #[tracing::instrument(fields(response_type = "ApiResponse"))]
    fn into_response(self) -> Response {
        match self {
            ApiResponse::None => (StatusCode::OK, success(&Some(()))),
            ApiResponse::NoneCreated => (StatusCode::CREATED, success(&Some(()))),
            ApiResponse::Token(t) => (StatusCode::OK, success(&json!({"token": t}))),
            ApiResponse::StockPriceVec(s) => (StatusCode::OK, success(&s)),
            ApiResponse::StockPortfolioVec(s) => (StatusCode::OK, success(&s)),
            ApiResponse::Balance(b) => (StatusCode::OK, success(&json!({"balance": b}))),
            ApiResponse::WalletVec(w) => (StatusCode::OK, success(&w)),
            ApiResponse::TradeVec(t) => (StatusCode::OK, success(&t)),
            ApiResponse::StockId(id) => (StatusCode::OK, success(&json!({"stock_id": id}))),
        }
        .into_response()
    }
}

#[derive(Debug)]
pub enum AppError {
    UsernameAlreadyTaken,
    UserDoesNotExist,
    AuthTokenInvalid,
    AuthTokenNotPresent,
    /// Generic DB error that is irrecoverable. Required: `error!()`
    DatabaseError,
    /// Error that should not happen/be possible. Required: `error!()`
    InternalServerError,
}

fn error(input: &'static str) -> String {
    json!({ "success": false, "data": { "error": input} }).to_string()
}

impl IntoResponse for AppError {
    #[tracing::instrument(fields(response_type = "AppError"))]
    fn into_response(self) -> Response {
        match self {
            AppError::UsernameAlreadyTaken => {
                (StatusCode::CONFLICT, error("Username already taken"))
            }
            AppError::UserDoesNotExist => (StatusCode::NOT_FOUND, error("User does not exist")),
            AppError::AuthTokenNotPresent => (
                StatusCode::UNAUTHORIZED,
                error("Authorization token not present"),
            ),
            AppError::AuthTokenInvalid => (
                StatusCode::UNAUTHORIZED,
                error("Authorization token not valid"),
            ),
            AppError::DatabaseError => (StatusCode::INTERNAL_SERVER_ERROR, error("")),
            AppError::InternalServerError => (StatusCode::INTERNAL_SERVER_ERROR, error("")),
        }
        .into_response()
    }
}
