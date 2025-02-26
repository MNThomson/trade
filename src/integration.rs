use std::assert_matches::assert_matches;

use axum::{
    body::Body,
    http::{self, Request, StatusCode},
    routing::RouterIntoService,
};
use http::request::Builder;
use pretty_assertions::assert_eq;
use serde::{Deserialize, Serialize, de};
use tower::{Service, ServiceExt};

use crate::{
    admin::{AddMoneyRequest, AddStockToUserRequest, CreateStockRequest},
    db::DB,
    order::{CancelStockTransactionRequest, PlaceStockOrderRequest},
    router,
    telemetry::tracing_init,
    types::{
        AppState, Balance, OrderStatus, OrderType, StockId, StockPortfolio, StockPortfolioVec,
        StockPrice, StockPriceVec, StockTransaction, TokenResponse, TradeVec, WalletTransaction,
        WalletVec,
    },
    user::{LoginRequest, RegisterRequest},
};

#[tokio::test]
async fn integration() {
    tracing_init(env!("CARGO_PKG_NAME"), env!("GIT_HASH"));

    let app = App::init().await;

    // Vanguard Register
    let sc = app
        .clone()
        .register(RegisterRequest {
            user_name: String::from("VanguardETF"),
            password: String::from("Vang@123"),
            name: String::from("Vanguard Corp."),
        })
        .await
        .unwrap();
    assert_eq!(sc, 201);

    // Vanguard username already taken
    let sc = app
        .clone()
        .register(RegisterRequest {
            user_name: String::from("VanguardETF"),
            password: String::from("Comp@124"),
            name: String::from("Vanguard Ltd."),
        })
        .await
        .unwrap_err();
    assert_eq!(sc, 400);

    // Vanguard Incorrect Password Login
    let sc = app
        .clone()
        .login(LoginRequest {
            user_name: String::from("VanguardETF"),
            password: String::from("Vang@1234"),
        })
        .await
        .unwrap_err();
    assert_eq!(sc, 400);

    // Vanguard Login
    let (sc, resp) = app
        .clone()
        .login(LoginRequest {
            user_name: String::from("VanguardETF"),
            password: String::from("Vang@123"),
        })
        .await
        .unwrap();
    let vanguard_token = resp.token;
    assert_eq!((sc, vanguard_token.len() > 10), (StatusCode::OK, true));

    // Create Google Stock
    let (sc, resp) = app
        .clone()
        .create_stock(
            &vanguard_token,
            CreateStockRequest {
                stock_name: String::from("Google"),
            },
        )
        .await
        .unwrap();
    let google_stock_id = resp.stock_id;
    assert_eq!(sc, 200);

    // Add 550 Google Stock to Vanguard
    let sc = app
        .clone()
        .add_stock_to_user(
            &vanguard_token,
            AddStockToUserRequest {
                stock_id: google_stock_id.clone(),
                quantity: 550,
            },
        )
        .await
        .unwrap();
    assert_eq!(sc, 200);

    // Create Apple Stock
    let (sc, resp) = app
        .clone()
        .create_stock(
            &vanguard_token,
            CreateStockRequest {
                stock_name: String::from("Apple"),
            },
        )
        .await
        .unwrap();
    let apple_stock_id = resp.stock_id;
    assert_eq!(sc, 200);

    // Add 350 Apple Stock to Vanguard
    let sc = app
        .clone()
        .add_stock_to_user(
            &vanguard_token,
            AddStockToUserRequest {
                stock_id: apple_stock_id.clone(),
                quantity: 350,
            },
        )
        .await
        .unwrap();
    assert_eq!(sc, 200);

    // Get Vanguard Stock Portfolio
    let (sc, resp) = app
        .clone()
        .get_stock_portfolio(&vanguard_token)
        .await
        .unwrap();
    assert_eq!(
        (sc, resp.0),
        (
            StatusCode::OK,
            vec![
                StockPortfolio {
                    stock_id: google_stock_id.clone(),
                    stock_name: String::from("Google"),
                    quantity_owned: 550
                },
                StockPortfolio {
                    stock_id: apple_stock_id.clone(),
                    stock_name: String::from("Apple"),
                    quantity_owned: 350
                },
            ]
        )
    );

    // Vanguard Sell 550 Google
    let sc = app
        .clone()
        .place_stock_order(
            &vanguard_token,
            PlaceStockOrderRequest {
                stock_id: google_stock_id.clone(),
                is_buy: false,
                order_type: OrderType::Limit,
                quantity: 550,
                price: Some(135),
            },
        )
        .await
        .unwrap();
    assert_eq!(sc, StatusCode::CREATED);

    // Vanguard Sell 350 Apple
    let sc = app
        .clone()
        .place_stock_order(
            &vanguard_token,
            PlaceStockOrderRequest {
                stock_id: apple_stock_id.clone(),
                is_buy: false,
                order_type: OrderType::Limit,
                quantity: 350,
                price: Some(140),
            },
        )
        .await
        .unwrap();
    assert_eq!(sc, StatusCode::CREATED);

    // Vanguard ensure they own no stock
    let (sc, resp) = app
        .clone()
        .get_stock_portfolio(&vanguard_token)
        .await
        .unwrap();
    assert_eq!((sc, resp.0), (StatusCode::OK, vec![]));

    // Vanguard get stock transactions
    let (sc, resp) = app
        .clone()
        .get_stock_transactions(&vanguard_token)
        .await
        .unwrap();
    let vanguard_google_stocktx_to_cancel = resp.0[0].stock_tx_id.clone();
    let vanguard_apple_stocktx_to_cancel = resp.0[1].stock_tx_id.clone();

    assert_matches!(
        &resp.0[..],
        [
            StockTransaction {
                //stock_id: google_stock_id,
                order_status: OrderStatus::InProgress,
                order_type: OrderType::Limit,
                is_buy: false,
                stock_price: 135,
                quantity: 550,
                ..
            },
            StockTransaction {
                //stock_id: apple_stock_id,
                order_status: OrderStatus::InProgress,
                order_type: OrderType::Limit,
                is_buy: false,
                stock_price: 140,
                quantity: 350,
                ..
            }
        ]
    );
    assert_eq!(sc, StatusCode::OK);

    // User1 Register
    let sc = app
        .clone()
        .register(RegisterRequest {
            user_name: String::from("FinanceGuru"),
            password: String::from("Fguru@2024"),
            name: String::from("The Finance Guru"),
        })
        .await
        .unwrap();
    assert_eq!(sc, 201);

    // User1 Login
    let (sc, resp) = app
        .clone()
        .login(LoginRequest {
            user_name: String::from("FinanceGuru"),
            password: String::from("Fguru@2024"),
        })
        .await
        .unwrap();
    let user1_token = resp.token;
    assert_eq!((sc, user1_token.len() > 10), (StatusCode::OK, true));

    // Get Stock Prices
    let (sc, resp) = app.clone().get_stock_prices(&user1_token).await.unwrap();
    assert_eq!(
        (sc, resp.0),
        (
            StatusCode::OK,
            vec![
                StockPrice {
                    stock_id: google_stock_id.clone(),
                    stock_name: "Google".to_string(),
                    current_price: 135
                },
                StockPrice {
                    stock_id: apple_stock_id.clone(),
                    stock_name: "Apple".to_string(),
                    current_price: 140
                }
            ]
        )
    );

    // User1 add money
    let sc = app
        .clone()
        .add_money_to_user(&user1_token, AddMoneyRequest { amount: 10_000 })
        .await
        .unwrap();
    assert_eq!(sc, StatusCode::CREATED);

    // User1 get wallet balance
    let (sc, resp) = app.clone().get_wallet_balance(&user1_token).await.unwrap();
    assert_eq!((sc, resp.balance), (StatusCode::OK, 10_000));

    // User1 buy 10 Google
    let sc = app
        .clone()
        .place_stock_order(
            &user1_token,
            PlaceStockOrderRequest {
                stock_id: google_stock_id.clone(),
                is_buy: true,
                order_type: OrderType::Market,
                quantity: 10,
                price: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(sc, StatusCode::CREATED);

    // User1 get stock transactions
    let (sc, resp) = app
        .clone()
        .get_stock_transactions(&user1_token)
        .await
        .unwrap();

    assert_matches!(
        &resp.0[..],
        [StockTransaction {
            order_status: OrderStatus::Completed,
            order_type: OrderType::Market,
            is_buy: true,
            stock_price: 135,
            quantity: 10,
            ..
        }]
    );
    assert_eq!(sc, StatusCode::OK);

    // User1 get wallet transactions
    let (sc, resp) = app
        .clone()
        .get_wallet_transactions(&user1_token)
        .await
        .unwrap();

    assert_matches!(
        &resp.0[..],
        [WalletTransaction {
            is_debit: true,
            amount: 1350,
            ..
        }]
    );
    assert_eq!(sc, StatusCode::OK);

    // User1 get wallet balance
    let (sc, resp) = app.clone().get_wallet_balance(&user1_token).await.unwrap();
    assert_eq!((sc, resp.balance), (StatusCode::OK, 8650));

    // User1 Stock Portfolio
    let (sc, resp) = app.clone().get_stock_portfolio(&user1_token).await.unwrap();
    assert_eq!(
        (sc, resp.0),
        (
            StatusCode::OK,
            vec![StockPortfolio {
                stock_id: google_stock_id.clone(),
                stock_name: String::from("Google"),
                quantity_owned: 10
            },]
        )
    );

    // Vanguard get stock transactions
    let (sc, resp) = app
        .clone()
        .get_stock_transactions(&vanguard_token)
        .await
        .unwrap();

    assert_matches!(
        &resp.0[..],
        [
            StockTransaction {
                //stock_id: google_stock_id,
                parent_stock_tx_id: None,
                order_status: OrderStatus::PartiallyComplete,
                order_type: OrderType::Limit,
                is_buy: false,
                stock_price: 135,
                quantity: 550,
                ..
            },
            StockTransaction {
                //stock_id: apple_stock_id,
                parent_stock_tx_id: None,
                order_status: OrderStatus::InProgress,
                order_type: OrderType::Limit,
                is_buy: false,
                stock_price: 140,
                quantity: 350,
                ..
            },
            StockTransaction {
                //stock_id: google_stock_id,
                parent_stock_tx_id: Some(..),
                order_status: OrderStatus::Completed,
                order_type: OrderType::Limit,
                is_buy: false,
                stock_price: 135,
                quantity: 10,
                ..
            }
        ]
    );
    assert_eq!(sc, StatusCode::OK);

    // Vanguard get wallet balance
    let (sc, resp) = app
        .clone()
        .get_wallet_balance(&vanguard_token)
        .await
        .unwrap();
    assert_eq!((sc, resp.balance), (StatusCode::OK, 1350));

    // Vanguard get wallet transactions
    let (sc, resp) = app
        .clone()
        .get_wallet_transactions(&vanguard_token)
        .await
        .unwrap();

    assert_matches!(
        &resp.0[..],
        [WalletTransaction {
            is_debit: false,
            amount: 1350,
            ..
        }]
    );
    assert_eq!(sc, StatusCode::OK);

    // User1 buy 20 Apple
    let sc = app
        .clone()
        .place_stock_order(
            &user1_token,
            PlaceStockOrderRequest {
                stock_id: apple_stock_id.clone(),
                is_buy: true,
                order_type: OrderType::Market,
                quantity: 20,
                price: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(sc, StatusCode::CREATED);

    // User1 get stock transactions
    let (sc, resp) = app
        .clone()
        .get_stock_transactions(&user1_token)
        .await
        .unwrap();

    assert_matches!(
        &resp.0[..],
        [
            StockTransaction {
                order_status: OrderStatus::Completed,
                order_type: OrderType::Market,
                is_buy: true,
                stock_price: 135,
                quantity: 10,
                ..
            },
            StockTransaction {
                order_status: OrderStatus::Completed,
                order_type: OrderType::Market,
                is_buy: true,
                stock_price: 140, // TODO: Spec says 120 but previously said sell APPL for 140
                quantity: 20,
                ..
            },
        ]
    );
    assert_eq!(sc, StatusCode::OK);

    // User1 get wallet transactions
    let (sc, resp) = app
        .clone()
        .get_wallet_transactions(&user1_token)
        .await
        .unwrap();

    assert_matches!(
        &resp.0[..],
        [
            WalletTransaction {
                is_debit: true,
                amount: 1350,
                ..
            },
            WalletTransaction {
                is_debit: true,
                amount: 2800, // TODO: this says 2400 but incorrect as APPL is selling for 140
                ..
            }
        ]
    );
    assert_eq!(sc, StatusCode::OK);

    // User1 get wallet balance
    let (sc, resp) = app.clone().get_wallet_balance(&user1_token).await.unwrap();
    assert_eq!((sc, resp.balance), (StatusCode::OK, 5850));
    // TODO: Also -400 off from pdf due to APPL price

    // User1 Stock Portfolio
    let (sc, resp) = app.clone().get_stock_portfolio(&user1_token).await.unwrap();
    assert_eq!(
        (sc, resp.0),
        (
            StatusCode::OK,
            vec![
                StockPortfolio {
                    stock_id: google_stock_id.clone(),
                    stock_name: String::from("Google"),
                    quantity_owned: 10
                },
                StockPortfolio {
                    stock_id: apple_stock_id.clone(),
                    stock_name: String::from("Apple"),
                    quantity_owned: 20
                },
            ]
        )
    );

    // User1 sell 5 Google
    let sc = app
        .clone()
        .place_stock_order(
            &user1_token,
            PlaceStockOrderRequest {
                stock_id: google_stock_id.clone(),
                is_buy: false,
                order_type: OrderType::Limit,
                quantity: 5,
                price: Some(130),
            },
        )
        .await
        .unwrap();
    assert_eq!(sc, StatusCode::CREATED);

    // User1 get stock transactions
    let (sc, resp) = app
        .clone()
        .get_stock_transactions(&user1_token)
        .await
        .unwrap();
    let user1_stocktx_to_cancel = resp.0[2].stock_tx_id.clone();

    assert_matches!(
        &resp.0[..],
        [
            StockTransaction {
                parent_stock_tx_id: None,
                order_status: OrderStatus::Completed,
                order_type: OrderType::Market,
                is_buy: true,
                stock_price: 135,
                quantity: 10,
                ..
            },
            StockTransaction {
                parent_stock_tx_id: None,
                order_status: OrderStatus::Completed,
                order_type: OrderType::Market,
                is_buy: true,
                stock_price: 140, // TODO: Spec says 120 but previously said sell APPL for 140
                quantity: 20,
                ..
            },
            StockTransaction {
                parent_stock_tx_id: None,
                order_status: OrderStatus::InProgress,
                order_type: OrderType::Limit,
                is_buy: false,
                stock_price: 130,
                quantity: 5,
                ..
            },
        ]
    );
    assert_eq!(sc, StatusCode::OK);

    // User1 get wallet transactions
    let (sc, resp) = app
        .clone()
        .get_wallet_transactions(&user1_token)
        .await
        .unwrap();

    assert_matches!(
        &resp.0[..],
        [
            WalletTransaction {
                is_debit: true,
                amount: 1350,
                ..
            },
            WalletTransaction {
                is_debit: true,
                amount: 2800, // TODO: this says 2400 but incorrect as APPL is selling for 140
                ..
            }
        ]
    );
    assert_eq!(sc, StatusCode::OK);

    // User1 get wallet balance
    let (sc, resp) = app.clone().get_wallet_balance(&user1_token).await.unwrap();
    assert_eq!((sc, resp.balance), (StatusCode::OK, 5850));
    // TODO: Also -400 off from pdf due to APPL price

    // User1 Stock Portfolio
    let (sc, resp) = app.clone().get_stock_portfolio(&user1_token).await.unwrap();
    assert_eq!(
        (sc, resp.0),
        (
            StatusCode::OK,
            vec![
                StockPortfolio {
                    stock_id: google_stock_id.clone(),
                    stock_name: String::from("Google"),
                    quantity_owned: 5,
                },
                StockPortfolio {
                    stock_id: apple_stock_id.clone(),
                    stock_name: String::from("Apple"),
                    quantity_owned: 20,
                },
            ]
        )
    );

    // Get Stock Prices
    let (sc, resp) = app.clone().get_stock_prices(&user1_token).await.unwrap();
    assert_eq!(
        (sc, resp.0),
        (
            StatusCode::OK,
            vec![
                StockPrice {
                    stock_id: google_stock_id.clone(),
                    stock_name: "Google".to_string(),
                    current_price: 130
                },
                StockPrice {
                    stock_id: apple_stock_id.clone(),
                    stock_name: "Apple".to_string(),
                    current_price: 140
                }
            ]
        )
    );

    // Vanguard buy 2 Google
    let sc = app
        .clone()
        .place_stock_order(
            &vanguard_token,
            PlaceStockOrderRequest {
                stock_id: google_stock_id.clone(),
                is_buy: true,
                order_type: OrderType::Market,
                quantity: 2,
                price: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(sc, StatusCode::CREATED);

    // Vanguard get stock transactions
    let (sc, resp) = app
        .clone()
        .get_stock_transactions(&vanguard_token)
        .await
        .unwrap();

    assert_matches!(
        &resp.0[..],
        [
            StockTransaction {
                //stock_id: google_stock_id,
                parent_stock_tx_id: None,
                order_status: OrderStatus::PartiallyComplete,
                order_type: OrderType::Limit,
                is_buy: false,
                stock_price: 135,
                quantity: 550,
                ..
            },
            StockTransaction {
                //stock_id: apple_stock_id,
                parent_stock_tx_id: None,
                order_status: OrderStatus::PartiallyComplete,
                order_type: OrderType::Limit,
                is_buy: false,
                stock_price: 140,
                quantity: 350,
                ..
            },
            StockTransaction {
                //stock_id: google_stock_id,
                parent_stock_tx_id: Some(..),
                wallet_tx_id: Some(..),
                order_status: OrderStatus::Completed,
                order_type: OrderType::Limit,
                is_buy: false,
                stock_price: 135,
                quantity: 10,
                ..
            },
            // TODO: This TX does not exist in pdf but should
            StockTransaction {
                //stock_id: apple,
                parent_stock_tx_id: Some(..),
                wallet_tx_id: Some(..),
                order_status: OrderStatus::Completed,
                order_type: OrderType::Limit,
                is_buy: false,
                stock_price: 140,
                quantity: 20,
                ..
            },
            StockTransaction {
                //stock_id: google_stock_id,
                parent_stock_tx_id: None,
                wallet_tx_id: Some(..),
                order_status: OrderStatus::Completed,
                order_type: OrderType::Market,
                is_buy: true,
                stock_price: 130,
                quantity: 2,
                ..
            },
        ]
    );
    assert_eq!(sc, StatusCode::OK);

    // Vanguard get wallet transactions
    let (sc, resp) = app
        .clone()
        .get_wallet_transactions(&vanguard_token)
        .await
        .unwrap();

    assert_matches!(
        &resp.0[..],
        [
            WalletTransaction {
                is_debit: false,
                amount: 1350,
                ..
            },
            WalletTransaction {
                is_debit: false,
                amount: 2800, // TODO: Also incorrect in pdf
                ..
            },
            WalletTransaction {
                is_debit: true,
                amount: 260,
                ..
            },
        ]
    );
    assert_eq!(sc, StatusCode::OK);

    // Vanguard get wallet balance
    let (sc, resp) = app
        .clone()
        .get_wallet_balance(&vanguard_token)
        .await
        .unwrap();
    assert_eq!((sc, resp.balance), (StatusCode::OK, 3890));
    // TODO: +400 off from pdf due to incorrect pricing

    // Vanguard buy 5 Google (and fail)
    let sc = app
        .clone()
        .place_stock_order(
            &vanguard_token,
            PlaceStockOrderRequest {
                stock_id: google_stock_id.clone(),
                is_buy: true,
                order_type: OrderType::Market,
                quantity: 5,
                price: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(sc, StatusCode::CREATED);

    // User1 cancel Google sell order
    let sc = app
        .clone()
        .cancel_stock_order(
            &user1_token,
            CancelStockTransactionRequest {
                stock_tx_id: user1_stocktx_to_cancel,
            },
        )
        .await
        .unwrap();
    assert_eq!(sc, StatusCode::OK);

    // Vanguard cancel Google sell order
    let sc = app
        .clone()
        .cancel_stock_order(
            &vanguard_token,
            CancelStockTransactionRequest {
                stock_tx_id: vanguard_google_stocktx_to_cancel,
            },
        )
        .await
        .unwrap();
    assert_eq!(sc, StatusCode::OK);

    // Vanguard cancel Apple sell order
    let sc = app
        .clone()
        .cancel_stock_order(
            &vanguard_token,
            CancelStockTransactionRequest {
                stock_tx_id: vanguard_apple_stocktx_to_cancel,
            },
        )
        .await
        .unwrap();
    assert_eq!(sc, StatusCode::OK);

    // Vanguard get stock transactions
    let (sc, resp) = app
        .clone()
        .get_stock_transactions(&vanguard_token)
        .await
        .unwrap();

    assert_matches!(
        &resp.0[..],
        [
            StockTransaction {
                //stock_id: google_stock_id,
                parent_stock_tx_id: None,
                order_status: OrderStatus::Cancelled,
                order_type: OrderType::Limit,
                is_buy: false,
                stock_price: 135,
                quantity: 550,
                ..
            },
            StockTransaction {
                //stock_id: apple_stock_id,
                parent_stock_tx_id: None,
                order_status: OrderStatus::Cancelled,
                order_type: OrderType::Limit,
                is_buy: false,
                stock_price: 140,
                quantity: 350,
                ..
            },
            StockTransaction {
                //stock_id: google_stock_id,
                parent_stock_tx_id: Some(..),
                wallet_tx_id: Some(..),
                order_status: OrderStatus::Completed,
                order_type: OrderType::Limit,
                is_buy: false,
                stock_price: 135,
                quantity: 10,
                ..
            },
            StockTransaction {
                //stock_id: apple,
                parent_stock_tx_id: Some(..),
                wallet_tx_id: Some(..),
                order_status: OrderStatus::Completed,
                order_type: OrderType::Limit,
                is_buy: false,
                stock_price: 140,
                quantity: 20,
                ..
            },
            StockTransaction {
                //stock_id: google_stock_id,
                parent_stock_tx_id: None,
                wallet_tx_id: Some(..),
                order_status: OrderStatus::Completed,
                order_type: OrderType::Market,
                is_buy: true,
                stock_price: 130,
                quantity: 2,
                ..
            },
        ]
    );
    assert_eq!(sc, StatusCode::OK);

    // Get Vanguard Stock Portfolio
    let (sc, resp) = app
        .clone()
        .get_stock_portfolio(&vanguard_token)
        .await
        .unwrap();
    assert_eq!(
        (sc, resp.0),
        (
            StatusCode::OK,
            vec![
                StockPortfolio {
                    stock_id: google_stock_id.clone(),
                    stock_name: String::from("Google"),
                    quantity_owned: 542
                },
                StockPortfolio {
                    stock_id: apple_stock_id.clone(),
                    stock_name: String::from("Apple"),
                    quantity_owned: 330
                },
            ]
        )
    );

    // User1 get stock transactions
    let (sc, resp) = app
        .clone()
        .get_stock_transactions(&user1_token)
        .await
        .unwrap();

    assert_matches!(
        &resp.0[..],
        [
            StockTransaction {
                parent_stock_tx_id: None,
                order_status: OrderStatus::Completed,
                order_type: OrderType::Market,
                is_buy: true,
                stock_price: 135,
                quantity: 10,
                ..
            },
            StockTransaction {
                parent_stock_tx_id: None,
                order_status: OrderStatus::Completed,
                order_type: OrderType::Market,
                is_buy: true,
                stock_price: 140, // TODO: Spec says 120 but previously said sell APPL for 140
                quantity: 20,
                ..
            },
            StockTransaction {
                parent_stock_tx_id: None,
                order_status: OrderStatus::Cancelled,
                order_type: OrderType::Limit,
                is_buy: false,
                stock_price: 130,
                quantity: 5,
                ..
            },
            StockTransaction {
                parent_stock_tx_id: Some(..),
                wallet_tx_id: Some(..),
                order_status: OrderStatus::Completed,
                order_type: OrderType::Limit,
                is_buy: false,
                stock_price: 130,
                quantity: 2,
                ..
            },
        ]
    );
    assert_eq!(sc, StatusCode::OK);

    // User1 Stock Portfolio
    let (sc, resp) = app.clone().get_stock_portfolio(&user1_token).await.unwrap();
    assert_eq!(
        (sc, resp.0),
        (
            StatusCode::OK,
            vec![
                StockPortfolio {
                    stock_id: google_stock_id.clone(),
                    stock_name: String::from("Google"),
                    quantity_owned: 8,
                },
                StockPortfolio {
                    stock_id: apple_stock_id.clone(),
                    stock_name: String::from("Apple"),
                    quantity_owned: 20,
                },
            ]
        )
    );

    // Invalid token get stock portfolio
    let sc = app
        .clone()
        .get_stock_portfolio(&String::from("asdasda"))
        .await
        .unwrap_err();
    assert_eq!(sc, StatusCode::UNAUTHORIZED);

    // Invalid token get wallet transaction
    let sc = app
        .clone()
        .get_wallet_transactions(&String::from("asdasda"))
        .await
        .unwrap_err();
    assert_eq!(sc, StatusCode::UNAUTHORIZED);

    // Invalid token get wallet transaction
    let sc = app
        .clone()
        .get_stock_transactions(&String::from("asdasda"))
        .await
        .unwrap_err();
    assert_eq!(sc, StatusCode::UNAUTHORIZED);

    // User1 add invalid money
    let sc = app
        .clone()
        .add_money_to_user(&user1_token, AddMoneyRequest { amount: -10_000 })
        .await
        .unwrap_err();
    assert_eq!(sc, StatusCode::BAD_REQUEST);

    // User1 invalid buy
    let sc = app
        .clone()
        .place_stock_order(
            &user1_token,
            PlaceStockOrderRequest {
                stock_id: apple_stock_id.clone(),
                is_buy: true,
                order_type: OrderType::Market,
                quantity: 20,
                price: Some(80),
            },
        )
        .await
        .unwrap_err();
    assert_eq!(sc, StatusCode::BAD_REQUEST);

    // Invalid user add money
    let sc = app
        .clone()
        .add_money_to_user(&String::from("ASDASDSAD"), AddMoneyRequest { amount: -100 })
        .await
        .unwrap_err();
    assert_eq!(sc, StatusCode::UNAUTHORIZED);

    // Invalid user buy
    let sc = app
        .clone()
        .place_stock_order(
            &String::from("DSFDSFFDS"),
            PlaceStockOrderRequest {
                stock_id: apple_stock_id.clone(),
                is_buy: true,
                order_type: OrderType::Market,
                quantity: 20,
                price: None,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(sc, StatusCode::UNAUTHORIZED);
}

#[derive(Serialize, Deserialize)]
struct ApiResponseWrapper<T> {
    success: bool,
    data: T,
}

#[derive(Clone)]
struct App {
    app: RouterIntoService<Body>,
}

impl App {
    async fn init() -> Self {
        let state = AppState {
            db: DB::init().await.unwrap(),
        };

        App {
            app: router(state).await.into_service(),
        }
    }

    async fn request<B: Serialize, R: for<'a> de::Deserialize<'a>>(
        mut self,
        token: &String,
        request: Builder,
        payload: Option<B>,
    ) -> Result<(StatusCode, R), StatusCode> {
        let request = request.header("token", token);
        let request = if let Some(ref p) = payload {
            request
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&p).unwrap()))
        } else {
            request.body(Body::empty())
        }
        .unwrap();

        let response = ServiceExt::<Request<Body>>::ready(&mut self.app)
            .await
            .unwrap()
            .call(request)
            .await
            .unwrap();

        let (_parts, rawbody) = response.into_parts();
        let bytes = axum::body::to_bytes(rawbody, usize::MAX).await.unwrap();
        let obj: ApiResponseWrapper<R> =
            serde_json::from_slice(&bytes).map_err(|_| _parts.status)?;

        Ok((_parts.status, obj.data))
    }

    async fn register(self, payload: RegisterRequest) -> Result<StatusCode, StatusCode> {
        let (sc, _resp) = self
            .request::<_, Option<i64>>(
                &"".to_string(),
                Request::builder()
                    .uri("/authentication/register")
                    .method("POST"),
                Some(payload),
            )
            .await?;

        Ok(sc)
    }

    async fn login(self, payload: LoginRequest) -> Result<(StatusCode, TokenResponse), StatusCode> {
        let resp = self
            .request::<_, TokenResponse>(
                &"".to_string(),
                Request::builder()
                    .uri("/authentication/login")
                    .method("POST"),
                Some(payload),
            )
            .await?;

        Ok(resp)
    }

    async fn create_stock(
        self,
        token: &String,
        payload: CreateStockRequest,
    ) -> Result<(StatusCode, StockId), StatusCode> {
        let resp = self
            .request::<_, StockId>(
                token,
                Request::builder().uri("/setup/createStock").method("POST"),
                Some(payload),
            )
            .await?;

        Ok(resp)
    }

    async fn add_stock_to_user(
        self,
        token: &String,
        payload: AddStockToUserRequest,
    ) -> Result<StatusCode, StatusCode> {
        let (sc, _resp) = self
            .request::<_, Option<i64>>(
                token,
                Request::builder()
                    .uri("/setup/addStockToUser")
                    .method("POST"),
                Some(payload),
            )
            .await?;

        Ok(sc)
    }

    async fn add_money_to_user(
        self,
        token: &String,
        payload: AddMoneyRequest,
    ) -> Result<StatusCode, StatusCode> {
        let (sc, _resp) = self
            .request::<_, Option<i64>>(
                token,
                Request::builder()
                    .uri("/transaction/addMoneyToWallet")
                    .method("POST"),
                Some(payload),
            )
            .await?;

        Ok(sc)
    }

    async fn get_stock_prices(
        self,
        token: &String,
    ) -> Result<(StatusCode, StockPriceVec), StatusCode> {
        let (sc, resp) = self
            .request::<_, StockPriceVec>(
                token,
                Request::builder().uri("/transaction/getStockPrices"),
                None::<i64>,
            )
            .await?;

        Ok((sc, resp))
    }

    async fn get_stock_portfolio(
        self,
        token: &String,
    ) -> Result<(StatusCode, StockPortfolioVec), StatusCode> {
        let (sc, resp) = self
            .request::<_, StockPortfolioVec>(
                token,
                Request::builder().uri("/transaction/getStockPortfolio"),
                None::<i64>,
            )
            .await?;

        Ok((sc, resp))
    }

    async fn get_wallet_balance(self, token: &String) -> Result<(StatusCode, Balance), StatusCode> {
        let (sc, resp) = self
            .request::<_, Balance>(
                token,
                Request::builder().uri("/transaction/getWalletBalance"),
                None::<i64>,
            )
            .await?;

        Ok((sc, resp))
    }

    async fn get_stock_transactions(
        self,
        token: &String,
    ) -> Result<(StatusCode, TradeVec), StatusCode> {
        let (sc, resp) = self
            .request::<_, TradeVec>(
                token,
                Request::builder().uri("/transaction/getStockTransactions"),
                None::<i64>,
            )
            .await?;

        Ok((sc, resp))
    }

    async fn get_wallet_transactions(
        self,
        token: &String,
    ) -> Result<(StatusCode, WalletVec), StatusCode> {
        let (sc, resp) = self
            .request::<_, WalletVec>(
                token,
                Request::builder().uri("/transaction/getWalletTransactions"),
                None::<i64>,
            )
            .await?;

        Ok((sc, resp))
    }

    async fn place_stock_order(
        self,
        token: &String,
        payload: PlaceStockOrderRequest,
    ) -> Result<StatusCode, StatusCode> {
        let (sc, _resp) = self
            .request::<_, Option<i64>>(
                token,
                Request::builder()
                    .uri("/engine/placeStockOrder")
                    .method("POST"),
                Some(payload),
            )
            .await?;

        Ok(sc)
    }

    async fn cancel_stock_order(
        self,
        token: &String,
        payload: CancelStockTransactionRequest,
    ) -> Result<StatusCode, StatusCode> {
        let (sc, _resp) = self
            .request::<_, Option<i64>>(
                token,
                Request::builder()
                    .uri("/engine/cancelStockTransaction")
                    .method("POST"),
                Some(payload),
            )
            .await?;

        Ok(sc)
    }
}
