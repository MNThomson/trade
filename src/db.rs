#[cfg(debug_assertions)]
use std::fs::remove_file;
use std::{fs::File, path::Path, time::Duration};

use chrono::DateTime;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use tracing::error;

use crate::types::{
    AppError, OrderStatus, OrderType, StockPortfolio, StockPrice, StockTransaction,
    WalletTransaction,
};

pub type DbPool = SqlitePool;

#[derive(Clone)]
pub struct DB {
    pool: DbPool,
}

pub const INIT_SQL: &str = include_str!("./init.sql");

#[cfg(not(debug_assertions))]
const PATH: &str = "data.db";

#[cfg(debug_assertions)]
const PATH: &str = "target/data.db";

impl DB {
    pub async fn init() -> Result<Self, ()> {
        #[cfg(debug_assertions)]
        {
            let _ = remove_file(PATH);
            let _ = remove_file(format!("{}-shm", PATH));
            let _ = remove_file(format!("{}-wal", PATH));
        }
        let should_init = !Path::new(PATH).exists();

        File::open(PATH).or_else(|_| File::create(PATH)).unwrap();
        let db = DB {
            pool: SqlitePoolOptions::new()
                .max_connections(50)
                .acquire_timeout(Duration::from_secs(3))
                .connect(format!("sqlite://{}", PATH).as_str())
                .await
                .unwrap(),
        };

        if should_init {
            let _ = sqlx::query(INIT_SQL).execute(&db.pool).await;
        }

        Ok(db)
    }

    pub async fn healthcheck(&self) -> Result<(), ()> {
        let row: (i64,) = sqlx::query_as("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .unwrap_or((0,));
        if row.0 == 1 { Ok(()) } else { Err(()) }
    }

    #[tracing::instrument(skip(self, password))]
    pub async fn create_user(&self, user_name: String, password: String) -> Result<(), AppError> {
        let res = sqlx::query!(
            "INSERT INTO users (user_name, password) VALUES (?, ?)",
            user_name,
            password
        )
        .execute(&self.pool)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(msg) if msg.message().contains("UNIQUE constraint failed:") => {
                AppError::UsernameAlreadyTaken
            }
            _ => {
                error!(user_name, "{}", &e);
                AppError::DatabaseError
            }
        })?;
        debug_assert_eq!(res.rows_affected(), 1);

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_user(&self, user_name: String) -> Result<DbUser, AppError> {
        let row = sqlx::query_as!(DbUser, "SELECT * FROM users WHERE user_name = ?", user_name)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match &e {
                sqlx::Error::RowNotFound => AppError::UserNotFound,
                _ => {
                    error!(user_name, "{}", &e);
                    AppError::DatabaseError
                }
            })?;

        Ok(row)
    }

    #[tracing::instrument(skip(self))]
    pub async fn add_money_to_user(&self, user_id: i64, amount: i64) -> Result<(), AppError> {
        let _row = sqlx::query!(
            "INSERT INTO deposits (user_id, amount) VALUES (?, ?)",
            user_id,
            amount
        )
        .execute(&self.pool)
        .await
        .map_err(|e| match &e {
            sqlx::Error::RowNotFound => AppError::UserNotFound,
            _ => {
                error!(user_id, "{}", &e);
                AppError::DatabaseError
            }
        })?;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn create_stock(&self, stock_name: String) -> Result<i64, AppError> {
        let stock_id = sqlx::query!(
            "INSERT INTO stocks (stock_name) VALUES (?) RETURNING stock_id",
            stock_name
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            error!(stock_name, "{}", &e);
            AppError::DatabaseError
        })?
        .stock_id;

        Ok(stock_id)
    }

    #[tracing::instrument(skip(self))]
    pub async fn add_stock_to_user(
        &self,
        user_id: i64,
        stock_id: i64,
        quantity: i64,
    ) -> Result<(), AppError> {
        let _ = sqlx::query!(r#"
            INSERT INTO orders (user_id, stock_id, amount, limit_price, order_status, created_at) VALUES (1, ?, ?, 0, ?, 0), (?, ?, ?, NULL, ?, 0);
            INSERT INTO trades (sell_order, buy_order, amount, created_at) VALUES ((SELECT order_id FROM orders WHERE user_id = 1 AND amount = ?), (SELECT order_id FROM orders WHERE user_id = ? AND amount = ?), ?, 0);"#,
            stock_id, quantity, OrderStatus::Completed as i64, user_id, stock_id, quantity, OrderStatus::Completed as i64, quantity, user_id, quantity, quantity
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!(user_id, stock_id, quantity, "{}", &e);
            AppError::DatabaseError
        })?;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_stock_prices(&self) -> Result<Vec<StockPrice>, AppError> {
        let data = sqlx::query_as!(
            DBStockPrice,
            r#"
            SELECT s.stock_id AS "stock_id!: i64", s.stock_name AS "stock_name!: String", MIN(o.limit_price) AS price
            FROM stocks s
            JOIN orders o ON s.stock_id = o.stock_id
            WHERE o.limit_price IS NOT NULL AND o.order_status IN (?, ?)
            GROUP BY s.stock_id, s.stock_name
            ORDER BY s.stock_name DESC
           "#,
            OrderStatus::InProgress as i64,
            OrderStatus::PartiallyComplete as i64
        )
        .fetch_all(&self.pool)
        .await
        .map(|p| {
            p.iter()
                .map(|i| StockPrice {
                    stock_id: i.stock_id.to_string(),
                    stock_name: i.stock_name.clone(),
                    current_price: i.price.unwrap_or(0),
                })
                .collect()
        })
        .map_err(|e| {
            error!("{}", &e);
            AppError::DatabaseError
        })?;

        Ok(data)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_wallet_balance(&self, user_id: i64) -> Result<i64, AppError> {
        let data = sqlx::query!(
            r#"
            WITH TotalDeposits AS (
                SELECT COALESCE(SUM(d.amount), 0) AS deposits_total
                FROM deposits d
                WHERE d.user_id = ?
            ),
            TotalTrades AS (
                SELECT COALESCE(SUM(CASE
                    WHEN os.user_id = ? THEN t.amount * os.limit_price
                    WHEN ob.user_id = ? AND ob.order_status <> ? THEN -t.amount * os.limit_price
                    ELSE 0 END
                ), 0) AS trades_total
                FROM trades t
                LEFT JOIN orders os ON os.order_id = t.sell_order
                LEFT JOIN orders ob ON ob.order_id = t.buy_order
                WHERE (os.user_id = ? OR ob.user_id = ?)
            )
            SELECT (deposits_total + trades_total) AS balance FROM TotalDeposits, TotalTrades;
           "#,
            user_id,
            user_id,
            user_id,
            OrderStatus::Failed as i64,
            user_id,
            user_id
        )
        .fetch_one(&self.pool)
        .await
        .map(|i| i.balance)
        .map_err(|e| {
            error!("{}", &e);
            AppError::DatabaseError
        })?;

        Ok(data)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_wallet_transactions(
        &self,
        user_id: i64,
    ) -> Result<Vec<WalletTransaction>, AppError> {
        let data = sqlx::query_as!(
            DBWalletTransaction,
            r#"
            SELECT t.trade_id AS wallet_tx_id, os.order_id AS stock_tx_id, (t.amount * os.limit_price) AS "amount!: i64", os.user_id AS seller_id, t.created_at AS time_stamp
            FROM trades t
            LEFT JOIN orders os ON os.order_id = t.sell_order
            LEFT JOIN orders ob ON ob.order_id = t.buy_order
            WHERE (os.user_id = ? OR ob.user_id = ?) AND os.created_at != 0 AND ob.created_at != 0
            ORDER BY t.created_at
           "#,
            user_id,
            user_id
        )
        .fetch_all(&self.pool)
        .await
        .map(|p| {
            p.iter()
                .map(|i| WalletTransaction {
                        wallet_tx_id: i.wallet_tx_id.to_string(),
                        stock_tx_id: i.stock_tx_id.to_string(),
                        is_debit: i.seller_id.ne(&user_id),
                        amount: i.amount,
                        time_stamp: DateTime::from_timestamp_millis(i.time_stamp).unwrap(),
                    })
                .collect()
        })
        .map_err(|e| {
            error!(user_id, "{}", &e);
            AppError::DatabaseError
        })?;

        Ok(data)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_stock_portfolio(&self, user_id: i64) -> Result<Vec<StockPortfolio>, AppError> {
        let data = sqlx::query_as!(
            DBStockPortfolio,
            r#"
            SELECT s.stock_id, s.stock_name,
                SUM(CASE
                    WHEN o.order_status = ? AND t.buy_order = o.order_id THEN t.amount -- All buy orders that haven't failed are complete
                    WHEN o.limit_price IS NOT NULL THEN CASE
                        WHEN o.order_status IN (?, ?, ?) THEN -o.amount
                        WHEN t.sell_order = o.order_id THEN -t.amount
                        ELSE 0 END
                    ELSE 0 END
                ) AS "quantity_owned!: i64"
            FROM stocks s
            LEFT JOIN orders o ON s.stock_id = o.stock_id
            LEFT JOIN trades t ON o.order_id = t.buy_order OR o.order_id = t.sell_order
            WHERE o.user_id = ?
            GROUP BY s.stock_id, s.stock_name;
           "#,
            OrderStatus::Completed as i64,
            OrderStatus::Completed as i64,
            OrderStatus::InProgress as i64,
            OrderStatus::PartiallyComplete as i64,
            user_id
        )
        .fetch_all(&self.pool)
        .await
        .map(|p| {
            p.iter()
                .map(|i| StockPortfolio {
                    stock_id: i.stock_id.to_string(),
                    stock_name: i.stock_name.clone(),
                    quantity_owned: i.quantity_owned,
                })
                .filter(|i| i.quantity_owned > 0)
                .collect()
        })
        .map_err(|e| {
            error!(user_id, "{}", &e);
            AppError::DatabaseError
        })?;

        Ok(data)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_stock_transactions(
        &self,
        user_id: i64,
    ) -> Result<Vec<StockTransaction>, AppError> {
        let data = sqlx::query_as!(
            DBStockTransaction,
            r#"
            SELECT o.order_id AS stock_tx_id, -1 AS parent_stock_tx_id, o.stock_id AS stock_id, o.order_status AS "order_status!: i64", o.limit_price AS stock_price, os.limit_price AS limit_price, o.amount AS quantity, o.created_at AS time_stamp
            FROM orders o
            LEFT JOIN trades t ON t.buy_order = o.order_id
            LEFT JOIN orders os ON os.order_id = t.sell_order
            WHERE o.user_id = ? AND o.created_at != 0
            UNION ALL
            SELECT t.trade_id AS stock_tx_id, os.order_id AS parent_stock_tx_id, os.stock_id, ? AS order_status, os.limit_price AS stock_price, 0 AS limit_price, t.amount AS quantity, t.created_at AS time_stamp
            FROM trades t
            JOIN orders ob ON ob.order_id = t.buy_order
            JOIN orders os ON os.order_id = t.sell_order
            WHERE (os.user_id = ? OR ob.user_id = ?) AND t.created_at != 0 AND (t.amount != ob.amount OR ob.user_id != ?)
            ORDER BY t.created_at
           "#,
            user_id,
            OrderStatus::Completed as i64,
            user_id,
            user_id,
            user_id
        ).
        fetch_all(&self.pool)
        .await
        .map(|p| {
            p.iter()
                .map(|i| StockTransaction{
                        stock_tx_id: i.stock_tx_id.to_string(),
                        parent_stock_tx_id: if i.parent_stock_tx_id>0 {Some(i.parent_stock_tx_id.to_string())} else {None},
                        stock_id: i.stock_id.to_string(),
                        wallet_tx_id: if i.parent_stock_tx_id > 0 {Some(i.stock_tx_id.to_string())} else {None},
                        order_status: i.order_status,
                        is_buy: i.stock_price.is_none(),
                        order_type: if i.stock_price.is_some() {OrderType::Limit} else {OrderType::Market},
                        stock_price: i.stock_price.unwrap_or_else(|| i.limit_price.expect("either stock_price or limit_price to exist")),
                        quantity: i.quantity,
                        time_stamp: DateTime::from_timestamp_millis(i.time_stamp).unwrap(),
                    } )
                .collect()
        })
        .map_err(|e| {
            error!(user_id, "{}", &e);
            AppError::DatabaseError
        })?;

        Ok(data)
    }

    #[tracing::instrument(skip(self))]
    pub async fn create_sell_order(
        &self,
        user_id: i64,
        stock_id: i64,
        quantity: i64,
        price: i64,
    ) -> Result<(), AppError> {
        let _ = sqlx::query!(r#"
            INSERT INTO orders (user_id, stock_id, amount, limit_price, order_status) VALUES (?, ?, ?, ?, ?);"#,
            user_id, stock_id, quantity, price, OrderStatus::InProgress as i64
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!(user_id, stock_id, quantity, "{}", &e);
            AppError::DatabaseError
        })?;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn create_buy_order(
        &self,
        user_id: i64,
        stock_id: i64,
        quantity: i64,
    ) -> Result<(), AppError> {
        let num = sqlx::query!(
            r#"
            WITH sold AS (
                SELECT COALESCE(SUM(o.amount),0) AS amount
                FROM trades t
                JOIN orders o ON t.sell_order = o.order_id
                WHERE o.user_id != ? AND o.stock_id = ? AND o.order_status IN (?,?)
            ),

            offered AS (
                SELECT COALESCE(SUM(o.amount),0) AS amount
                FROM orders o
                WHERE o.limit_price IS NOT NULL AND o.user_id != ? AND o.stock_id = ? AND o.order_status IN (?, ?)
            )

            SELECT (offered.amount - sold.amount) as amount FROM sold, offered;
            "#,
            user_id,
            stock_id,
            OrderStatus::InProgress as i64,
            OrderStatus::PartiallyComplete as i64,
            //
            user_id,
            stock_id,
            OrderStatus::InProgress as i64,
            OrderStatus::PartiallyComplete as i64,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            error!(user_id, stock_id, quantity, "{}", &e);
            AppError::DatabaseError
        })?
        .amount;
        // Not enough sell orders to fulfill this buy order
        if num < quantity {
            return Ok(());
        }

        let _ = sqlx::query!(
                    r#"
                    BEGIN TRANSACTION;

                    INSERT INTO orders (user_id, stock_id, amount, order_status) VALUES (?, ?, ?, ?);

                    WITH CheapestSellOrder AS (
                        SELECT order_id, amount, limit_price, user_id
                        FROM orders
                        WHERE stock_id = ? AND order_status IN (?, ?) AND user_id <> ?
                        ORDER BY limit_price ASC, created_at ASC
                        LIMIT 1
                    ),
                    BuyOrder AS (
                        SELECT order_id
                        FROM orders
                        WHERE user_id = ? AND stock_id = ? AND amount = ?
                    )
                    INSERT INTO trades (sell_order, buy_order, amount)
                    SELECT CheapestSellOrder.order_id, BuyOrder.order_id, ?
                    FROM CheapestSellOrder, BuyOrder;

                    WITH CheapestSellOrder AS (
                        SELECT order_id, amount, limit_price, user_id
                        FROM orders
                        WHERE stock_id = ? AND order_status IN (?, ?) AND user_id <> ?
                        ORDER BY limit_price ASC, created_at ASC
                        LIMIT 1
                    )
                    UPDATE orders
                    SET order_status = CASE WHEN amount = ? THEN ? ELSE ? END
                    WHERE order_id = (SELECT order_id FROM CheapestSellOrder);

                    COMMIT;
            "#,
                    user_id, stock_id, quantity, OrderStatus::Completed as i64,
                    //
                    stock_id, OrderStatus::InProgress as i64, OrderStatus::PartiallyComplete as i64, user_id,
                    //
                    user_id, stock_id, quantity,
                    //
                    quantity,
                    //
                    stock_id, OrderStatus::InProgress as i64, OrderStatus::PartiallyComplete as i64, user_id,
                    //
                    quantity, OrderStatus::Completed as i64, OrderStatus::PartiallyComplete as i64,
                )
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    error!(user_id, stock_id, quantity, "{}", &e);
                    AppError::DatabaseError
                })?;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn cancel_sell_order(&self, user_id: i64, stock_tx_id: i64) -> Result<(), AppError> {
        let _ = sqlx::query!(
            r#"
            UPDATE orders SET order_status = ? WHERE order_id = ? AND user_id = ? AND limit_price IS NOT NULL AND order_status > 0 RETURNING order_id;
            "#,
            OrderStatus::Cancelled as i64,
            stock_tx_id,
            user_id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            error!(user_id, stock_tx_id, "{}", &e);
            AppError::DatabaseError
        })?
        .map(|i| i.order_id)
        .ok_or(AppError::StockTransactionNotFound)?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct DbUser {
    pub user_id: i64,
    pub user_name: String,
    pub password: String,
    pub created_at: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct DBStockPrice {
    stock_id: i64,
    stock_name: String,
    price: Option<i64>,
}

#[derive(Debug, sqlx::FromRow)]
struct DBStockPortfolio {
    stock_id: i64,
    stock_name: String,
    quantity_owned: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct DBWalletTransaction {
    wallet_tx_id: i64,
    stock_tx_id: i64,
    seller_id: i64,
    amount: i64,
    time_stamp: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct DBStockTransaction {
    stock_tx_id: i64,
    parent_stock_tx_id: i64,
    stock_id: i64,
    order_status: OrderStatus,
    stock_price: Option<i64>,
    limit_price: Option<i64>,
    quantity: i64,
    time_stamp: i64,
}
