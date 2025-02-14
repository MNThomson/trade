use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use fake::{Dummy, faker::company::en::CompanyName};
use serde::{Deserialize, Serialize};
use serde_json::{self, json};

use crate::DB;

#[derive(Clone)]
pub struct AppState {
    pub db: DB,
}

#[derive(Serialize, Deserialize, Debug, Dummy)]
pub struct StockPrice {
    pub stock_id: String,
    #[dummy(faker = "CompanyName()")]
    pub stock_name: String,
    #[dummy(faker = "1..200")]
    pub current_price: usize,
}

#[derive(Serialize, Deserialize, Debug, Dummy, PartialEq)]
pub struct StockPortfolio {
    pub stock_id: String,
    #[dummy(faker = "CompanyName()")]
    pub stock_name: String,
    #[dummy(faker = "1..1000")]
    pub quantity_owned: i64,
}

#[derive(Serialize, Deserialize, Debug, Dummy)]
pub struct WalletTransaction {
    pub wallet_tx_id: String,
    pub stock_tx_id: String,
    pub is_debit: bool,
    #[dummy(faker = "1..10000")]
    pub amount: usize,
    pub time_stamp: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Dummy, PartialEq, Clone, Copy)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    Failed = -2,
    Cancelled = -1,
    Completed = 0,
    InProgress = 1,
    PartiallyComplete = 2,
}

impl From<i64> for OrderStatus {
    fn from(value: i64) -> Self {
        match value {
            -2 => OrderStatus::Failed,
            -1 => OrderStatus::Cancelled,
            0 => OrderStatus::Completed,
            1 => OrderStatus::InProgress,
            2 => OrderStatus::PartiallyComplete,
            _ => panic!("Invalid i64 value for OrderStatus"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Dummy, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum OrderType {
    Market,
    Limit,
}

#[derive(Serialize, Deserialize, Debug, Dummy, PartialEq)]
pub struct StockTransaction {
    pub stock_tx_id: String,
    pub parent_stock_tx_id: Option<String>,
    pub stock_id: String,
    pub wallet_tx_id: Option<String>,
    pub order_status: OrderStatus,
    pub is_buy: bool,
    pub order_type: OrderType,
    #[dummy(faker = "1..200")]
    pub stock_price: i64,
    #[dummy(faker = "1..200")]
    pub quantity: i64,
    pub time_stamp: DateTime<Utc>,
}

/////////////////////
/// API Responses ///
/////////////////////
fn success<T: Serialize>(input: &T) -> String {
    json!({ "success": true, "data": input }).to_string()
}

#[derive(Serialize, Deserialize)]
pub struct EmptyResponse {}

impl IntoResponse for EmptyResponse {
    #[tracing::instrument(skip_all)]
    fn into_response(self) -> Response {
        (StatusCode::OK, success(&None::<i64>)).into_response()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EmptyCreatedResponse {}

impl IntoResponse for EmptyCreatedResponse {
    #[tracing::instrument(skip_all)]
    fn into_response(self) -> Response {
        (StatusCode::CREATED, success(&None::<i64>)).into_response()
    }
}
macro_rules! impl_into_response {
    ($struct_name:ident) => {
        impl IntoResponse for $struct_name {
            #[tracing::instrument(skip_all)]
            fn into_response(self) -> Response {
                (StatusCode::OK, success(&self)).into_response()
            }
        }
    };
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TokenResponse {
    pub token: String,
}
impl_into_response!(TokenResponse);

#[derive(Serialize, Deserialize, Debug)]
pub struct StockPriceVec(pub Vec<StockPrice>);
impl_into_response!(StockPriceVec);

#[derive(Serialize, Deserialize, Debug)]
pub struct StockPortfolioVec(pub Vec<StockPortfolio>);
impl_into_response!(StockPortfolioVec);

#[derive(Serialize, Deserialize, Debug)]
pub struct Balance {
    pub balance: u64,
}
impl_into_response!(Balance);

#[derive(Serialize, Deserialize, Debug)]
pub struct WalletVec(pub Vec<WalletTransaction>);
impl_into_response!(WalletVec);

#[derive(Serialize, Deserialize, Debug)]
pub struct TradeVec(pub Vec<StockTransaction>);
impl_into_response!(TradeVec);

#[derive(Serialize, Deserialize, Debug)]
pub struct StockId {
    pub stock_id: String,
}
impl_into_response!(StockId);

#[derive(Debug)]
pub enum AppError {
    UsernameAlreadyTaken,
    UserNotFound,
    PasswordInvalid,
    AuthTokenInvalid,
    AuthTokenNotPresent,
    StockNotFound,
    /// Generic DB error that is irrecoverable. Required: `error!()`
    DatabaseError,
    /// Error that should not happen/be possible. Required: `error!()`
    InternalServerError,
}

fn error(input: &'static str) -> String {
    json!({ "success": false, "data": { "error": input } }).to_string()
}

impl IntoResponse for AppError {
    #[tracing::instrument(fields(response_type = "AppError"))]
    fn into_response(self) -> Response {
        match self {
            AppError::UsernameAlreadyTaken => {
                (StatusCode::CONFLICT, error("Username already taken"))
            }
            AppError::PasswordInvalid | AppError::UserNotFound => (
                StatusCode::UNAUTHORIZED,
                error("Username/Password combination incorrect"),
            ),
            AppError::AuthTokenNotPresent => (
                StatusCode::UNAUTHORIZED,
                error("Authorization token not present"),
            ),
            AppError::AuthTokenInvalid => (
                StatusCode::UNAUTHORIZED,
                error("Authorization token not valid"),
            ),
            AppError::StockNotFound => (StatusCode::NOT_FOUND, error("Stock not found")),
            AppError::DatabaseError => (StatusCode::INTERNAL_SERVER_ERROR, error("")),
            AppError::InternalServerError => (StatusCode::INTERNAL_SERVER_ERROR, error("")),
        }
        .into_response()
    }
}
