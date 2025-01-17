use axum::extract::State;

use crate::{
    AppState,
    types::{
        ApiResponse, OrderStatus, OrderType, StockPortfolio, StockPrice, StockTransaction,
        WalletTransaction,
    },
};

#[tracing::instrument(skip_all)]
pub async fn get_stock_prices(State(_state): State<AppState>) -> ApiResponse {
    ApiResponse::StockPriceVec(vec![
        StockPrice {
            stock_id: "asdafudsfsdjfls".into(),
            stock_name: "Apple".into(),
            current_price: 100,
        },
        StockPrice {
            stock_id: "lifdoijfdkjfdskj".into(),
            stock_name: "Google".into(),
            current_price: 200,
        },
    ])
}

#[tracing::instrument(skip_all)]
pub async fn get_stock_portfolio(State(_state): State<AppState>) -> ApiResponse {
    ApiResponse::StockPortfolioVec(vec![
        StockPortfolio {
            stock_id: "asdafudsfsdjfls".into(),
            stock_name: "Apple".into(),
            quantity_owned: 100,
        },
        StockPortfolio {
            stock_id: "lifdoijfdkjfdskj".into(),
            stock_name: "Google".into(),
            quantity_owned: 150,
        },
    ])
}

#[tracing::instrument(skip_all)]
pub async fn get_wallet_balance(State(_state): State<AppState>) -> ApiResponse {
    ApiResponse::Balance(100)
}

#[tracing::instrument(skip_all)]
pub async fn get_wallet_transactions(State(_state): State<AppState>) -> ApiResponse {
    ApiResponse::WalletVec(vec![
        WalletTransaction {
            wallet_tx_id: "628ba23df2210df6c3764823".into(),
            stock_tx_id: "62738363a50350b1fbb243a6".into(),
            is_debit: true,
            amount: 100,
            time_stamp: "2024-01-12T15:03:25.019+00:00".into(),
        },
        WalletTransaction {
            wallet_tx_id: "628ba36cf2210df6c3764824".into(),
            stock_tx_id: "62738363a50350b1fbb243a6".into(),
            is_debit: false,
            amount: 200,
            time_stamp: "2024-01-12T14:13:25.019+00:00".into(),
        },
    ])
}

#[tracing::instrument(skip_all)]
pub async fn get_stock_transactions(State(_state): State<AppState>) -> ApiResponse {
    ApiResponse::TradeVec(vec![
        StockTransaction {
            stock_tx_id: "62738363a50350b1fbb243a6".into(),
            stock_id: "asdafudsfsdjfls".into(),
            wallet_tx_id: "628ba23df2210df6c3764823".into(),
            order_status: OrderStatus::COMPLETED,
            is_buy: true,
            order_type: OrderType::LIMIT,
            stock_price: 50,
            quantity: 2,
            time_stamp: "2024-01-12T15:03:25.019+00:00".into(),
        },
        StockTransaction {
            stock_tx_id: "62738363a50350b1fbb243a6".into(),
            stock_id: "sidofSFDjslkdfj".into(),
            wallet_tx_id: "628ba36cf2210df6c3764824".into(),
            order_status: OrderStatus::COMPLETED,
            is_buy: false,
            order_type: OrderType::MARKET,
            stock_price: 100,
            quantity: 2,
            time_stamp: "2024-01-12T14:13:25.019+00:00".into(),
        },
    ])
}
