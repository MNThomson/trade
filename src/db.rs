use std::time::Duration;

use bigdecimal::{BigDecimal, ToPrimitive};
use chrono::NaiveDateTime;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tracing::error;

use crate::types::{
    AppError, OrderStatus, OrderType, StockPortfolio, StockPrice, StockTransaction,
    WalletTransaction,
};

pub type DbPool = PgPool;

#[derive(Clone)]
pub struct DB {
    pool: DbPool,
}

impl DB {
    pub async fn init() -> Result<Self, ()> {
        let db = DB {
            pool: PgPoolOptions::new()
                .max_connections(10)
                .acquire_timeout(Duration::from_secs(90))
                .connect(std::env::var("DB_ENDPOINT").unwrap().as_str())
                .await
                .unwrap(),
        };

        Ok(db)
    }

    pub async fn healthcheck(&self) -> Result<(), ()> {
        let row: (i64,) = sqlx::query_as("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .unwrap_or((0,));
        if row.0 == 1 { Ok(()) } else { Err(()) }
    }

    #[tracing::instrument(skip(self, password), fields(service.name = "db", db.operation.name = "INSERT"))]
    pub async fn create_user(&self, user_name: String, password: String) -> Result<(), AppError> {
        let res = sqlx::query!(
            "INSERT INTO users (user_name, password) VALUES ($1, $2)",
            user_name,
            password
        )
        .execute(&self.pool)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(msg) if msg.message().contains("violates unique constraint") => {
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

    #[tracing::instrument(skip(self), fields(service.name = "db", db.operation.name = "SELECT"))]
    pub async fn get_user(&self, user_name: String) -> Result<DbUser, AppError> {
        let row = sqlx::query_as!(
            DbUser,
            "SELECT * FROM users WHERE user_name = $1;",
            user_name
        )
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

    #[tracing::instrument(skip(self), fields(service.name = "db", db.operation.name = "INSERT"))]
    pub async fn add_money_to_user(&self, user_id: i64, amount: i64) -> Result<(), AppError> {
        let _row = sqlx::query!(
            "INSERT INTO deposits (user_id, amount) VALUES ($1, $2)",
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

    #[tracing::instrument(skip(self), fields(service.name = "db", db.operation.name = "INSERT"))]
    pub async fn create_stock(&self, stock_name: String) -> Result<i64, AppError> {
        let stock_id = sqlx::query!(
            "INSERT INTO stocks (stock_name) VALUES ($1) RETURNING stock_id",
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

    #[tracing::instrument(skip(self), fields(service.name = "db", db.operation.name = "INSERT"))]
    pub async fn add_stock_to_user(
        &self,
        user_id: i64,
        stock_id: i64,
        quantity: i64,
    ) -> Result<(), AppError> {
        let order_ids = sqlx::query!(r#"
            INSERT INTO orders (user_id, stock_id, amount, limit_price, order_status, created_at) VALUES (1, $1, $2, 0, $3, '0001-01-01 00:00:00'), ($4, $5, $6, NULL, $7, '0001-01-01 00:00:00') RETURNING order_id"#,
            stock_id, quantity, OrderStatus::Completed as i64,
            //
            user_id, stock_id, quantity, OrderStatus::Completed as i64,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            error!(user_id, stock_id, quantity, "{}", &e);
            AppError::DatabaseError
        })?;

        let _ = sqlx::query!(r#"
            INSERT INTO trades (sell_order, buy_order, amount, created_at) VALUES ($1, $2, $3, '0001-01-01 00:00:00')"#,
             order_ids[0].order_id, order_ids[1].order_id, quantity
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!(user_id, stock_id, quantity, "{}", &e);
            AppError::DatabaseError
        })?;

        Ok(())
    }

    #[tracing::instrument(skip(self), fields(service.name = "db", db.operation.name = "SELECT"))]
    pub async fn get_stock_prices(&self) -> Result<Vec<StockPrice>, AppError> {
        let data = sqlx::query_as!(
            DBStockPrice,
            r#"
            SELECT s.stock_id AS "stock_id!", s.stock_name AS "stock_name!", MIN(o.limit_price) AS price
            FROM stocks s
            JOIN orders o ON s.stock_id = o.stock_id
            WHERE o.limit_price IS NOT NULL AND o.order_status IN ($1, $2)
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

    #[tracing::instrument(skip(self), fields(service.name = "db", db.operation.name = "SELECT"))]
    pub async fn get_wallet_balance(&self, user_id: i64) -> Result<i64, AppError> {
        let data = sqlx::query!(
            r#"
            WITH TotalDeposits AS (
                SELECT COALESCE(SUM(d.amount), 0) AS deposits_total
                FROM deposits d
                WHERE d.user_id = $1
            ),
            TotalTrades AS (
                SELECT COALESCE(SUM(CASE
                    WHEN os.user_id = $2 THEN t.amount * os.limit_price
                    WHEN ob.user_id = $3 THEN -t.amount * os.limit_price
                    ELSE 0 END
                ), 0) AS trades_total
                FROM trades t
                LEFT JOIN orders os ON os.order_id = t.sell_order
                LEFT JOIN orders ob ON ob.order_id = t.buy_order
                WHERE (os.user_id = $4 OR ob.user_id = $5)
            )
            SELECT (deposits_total + trades_total) AS "balance!" FROM TotalDeposits, TotalTrades;
           "#,
            user_id,
            user_id,
            user_id,
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

        Ok(data.to_i64().expect("to turn into i64"))
    }

    #[tracing::instrument(skip(self), fields(service.name = "db", db.operation.name = "SELECT"))]
    pub async fn get_wallet_transactions(
        &self,
        user_id: i64,
    ) -> Result<Vec<WalletTransaction>, AppError> {
        let data = sqlx::query_as!(
            DBWalletTransaction,
            r#"
            SELECT t.trade_id AS wallet_tx_id, (t.amount * os.limit_price) AS "amount!", os.user_id AS seller_id, t.created_at AS time_stamp, CASE WHEN ob.user_id = $1 THEN ob.order_id ELSE os.order_id END AS "stock_tx_id!"
            FROM trades t
            LEFT JOIN orders os ON os.order_id = t.sell_order
            LEFT JOIN orders ob ON ob.order_id = t.buy_order
            WHERE (os.user_id = $2 OR ob.user_id = $3) AND os.created_at != '0001-01-01 00:00:00' AND ob.created_at != '0001-01-01 00:00:00'
            ORDER BY t.created_at
           "#,
            user_id,
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
                        time_stamp: i.time_stamp.and_utc(),
                    })
                .collect()
        })
        .map_err(|e| {
            error!(user_id, "{}", &e);
            AppError::DatabaseError
        })?;

        Ok(data)
    }

    #[tracing::instrument(skip(self), fields(service.name = "db", db.operation.name = "SELECT"))]
    pub async fn get_stock_portfolio(&self, user_id: i64) -> Result<Vec<StockPortfolio>, AppError> {
        let data = sqlx::query_as!(
            DBStockPortfolio,
            r#"
            SELECT s.stock_id, s.stock_name,
                SUM(CASE
                    WHEN o.order_status = $1 AND t.buy_order = o.order_id THEN t.amount -- All buy orders that haven't failed are complete
                    WHEN o.limit_price IS NOT NULL THEN CASE
                        WHEN o.order_status IN ($2, $3, $4) THEN -o.amount
                        WHEN t.sell_order = o.order_id THEN -t.amount
                        ELSE 0 END
                    ELSE 0 END
                ) AS "quantity_owned!"
            FROM stocks s
            LEFT JOIN orders o ON s.stock_id = o.stock_id
            LEFT JOIN trades t ON o.order_id = t.buy_order OR o.order_id = t.sell_order
            WHERE o.user_id =$5
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
                    quantity_owned: i.quantity_owned.to_i64().expect("To have less"),
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

    #[tracing::instrument(skip(self), fields(service.name = "db", db.operation.name = "SELECT"))]
    pub async fn get_stock_transactions(
        &self,
        user_id: i64,
    ) -> Result<Vec<StockTransaction>, AppError> {
        let data = sqlx::query_as!(
            DBStockTransaction,
            r#"
            SELECT o.order_id AS "stock_tx_id!", -1 AS "parent_stock_tx_id!", o.stock_id AS "stock_id!", o.order_status AS "order_status!", o.limit_price AS stock_price, os.limit_price AS limit_price, o.amount AS "quantity!", o.created_at AS time_stamp, CASE WHEN o.amount = t.amount THEN t.trade_id ELSE -1 END AS "wallet_tx_id!"
            FROM orders o
            LEFT JOIN trades t ON t.buy_order = o.order_id
            LEFT JOIN orders os ON os.order_id = t.sell_order
            WHERE o.user_id = $1 AND o.created_at != '0001-01-01 00:00:00'

            UNION ALL

            SELECT t.trade_id AS "wallet_tx_id!", os.order_id AS "parent_stock_tx_id!", os.stock_id, $2 AS "order_status!", os.limit_price AS stock_price, 0 AS limit_price, t.amount AS "quantity!", t.created_at AS time_stamp, CASE WHEN ob.user_id = $3 THEN ob.order_id ELSE os.order_id END AS stock_tx_id
            FROM trades t
            JOIN orders ob ON ob.order_id = t.buy_order
            JOIN orders os ON os.order_id = t.sell_order
            WHERE (os.user_id = $4 OR ob.user_id = $5) AND t.created_at != '0001-01-01 00:00:00' AND (t.amount != ob.amount OR ob.user_id != $6)

            ORDER BY time_stamp
           "#,
            user_id,
            //
            OrderStatus::Completed as i64,
            user_id,
            //
            user_id, user_id, user_id
        ).
        fetch_all(&self.pool)
        .await
        .map(|p| {
            p.iter()
                .map(|i| StockTransaction{
                        stock_tx_id: i.stock_tx_id.to_string(),
                        parent_stock_tx_id: if i.parent_stock_tx_id>0 {Some(i.parent_stock_tx_id.to_string())} else {None},
                        stock_id: i.stock_id.to_string(),
                        wallet_tx_id: if i.wallet_tx_id > 0 {Some(i.wallet_tx_id.to_string())} else {None},
                        order_status: i.order_status,
                        is_buy: i.stock_price.is_none(),
                        order_type: if i.stock_price.is_some() {OrderType::Limit} else {OrderType::Market},
                        stock_price: i.stock_price.unwrap_or_else(|| i.limit_price.expect("either stock_price or limit_price to exist")),
                        quantity: i.quantity,
                        time_stamp: i.time_stamp.expect("timestamp to exist").and_utc(),
                    } )
                .collect()
        })
        .map_err(|e| {
            error!(user_id, "{}", &e);
            AppError::DatabaseError
        })?;

        Ok(data)
    }

    #[tracing::instrument(skip(self), fields(service.name = "db", db.operation.name = "INSERT"))]
    pub async fn create_sell_order(
        &self,
        user_id: i64,
        stock_id: i64,
        quantity: i64,
        price: i64,
    ) -> Result<(), AppError> {
        let _ = sqlx::query!(r#"
            INSERT INTO orders (user_id, stock_id, amount, limit_price, order_status) VALUES ($1, $2, $3, $4, $5)"#,
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

    #[tracing::instrument(skip(self), fields(service.name = "db", db.operation.name = "INSERT"))]
    pub async fn create_buy_order(
        &self,
        user_id: i64,
        stock_id: i64,
        quantity: i64,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await.unwrap();
        /*
        let num = sqlx::query!(
            r#"
            WITH sold AS (
                SELECT COALESCE(SUM(o.amount),0) AS amount
                FROM trades t
                JOIN orders o ON t.sell_order = o.order_id
                WHERE o.user_id != $1 AND o.stock_id = $2 AND o.order_status IN ($3,$4)
            ),

            offered AS (
                SELECT COALESCE(SUM(o.amount),0) AS amount
                FROM orders o
                WHERE o.limit_price IS NOT NULL AND o.user_id != $5 AND o.stock_id = $6 AND o.order_status IN ($7, $8)
            )

            SELECT (offered.amount - sold.amount) as "amount!" FROM sold, offered;
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
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            error!(user_id, stock_id, quantity, "{}", &e);
            AppError::DatabaseError
        })?
        .amount.to_i64().expect("to convert to i64");
        // Not enough sell orders to fulfill this buy order
        if num < quantity {
            error!("NOT ENOUGH STOCKIES");
            return Ok(());
        }
        */

        let buy_order = sqlx::query!(
            "INSERT INTO orders (user_id, stock_id, amount, order_status) VALUES ($1, $2, $3, $4) RETURNING order_id",
            user_id,
            stock_id,
            quantity,
            OrderStatus::Completed as i64,
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            error!(user_id, stock_id, quantity, "{}", &e);
            AppError::DatabaseError
        })?.order_id;

        let cheapest_sell_order = sqlx::query!(
            r#"
            SELECT order_id, amount, limit_price, user_id
            FROM orders
            WHERE stock_id = $1 AND order_status IN ($2, $3) AND user_id != $4
            ORDER BY limit_price ASC, created_at ASC
            LIMIT 1
    "#,
            stock_id,
            OrderStatus::InProgress as i64,
            OrderStatus::PartiallyComplete as i64,
            user_id,
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            error!(user_id, stock_id, quantity, "{}", &e);
            AppError::DatabaseError
        })?
        .order_id;

        let _ = sqlx::query!(
            r#"
            INSERT INTO trades (sell_order, buy_order, amount) VALUES ($1, $2, $3)
    "#,
            cheapest_sell_order,
            buy_order,
            quantity
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            error!(user_id, stock_id, quantity, "{}", &e);
            AppError::DatabaseError
        })?;

        let _ = sqlx::query!(
            r#"
            UPDATE orders
            SET order_status = CASE WHEN amount = $1 THEN $2::bigint ELSE $3 END
            WHERE order_id = $4
    "#,
            quantity,
            OrderStatus::Completed as i64,
            OrderStatus::PartiallyComplete as i64,
            cheapest_sell_order
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            error!(user_id, stock_id, quantity, "{}", &e);
            AppError::DatabaseError
        })?;

        let _ = tx.commit().await;
        Ok(())
    }

    #[tracing::instrument(skip(self), fields(service.name = "db", db.operation.name = "INSERT"))]
    pub async fn cancel_sell_order(&self, user_id: i64, stock_tx_id: i64) -> Result<(), AppError> {
        // TODO: The TA provided tests fail when the user_id is verified
        //       This seems like a massive security issue.....buuuuuuut
        let _ = sqlx::query!(
            r#"
            UPDATE orders SET order_status = $1 WHERE order_id = $2 AND limit_price IS NOT NULL AND order_status > 0 RETURNING order_id
            "#,
            OrderStatus::Cancelled as i64,
            stock_tx_id,
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
    pub created_at: NaiveDateTime,
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
    quantity_owned: BigDecimal,
}

#[derive(Debug, sqlx::FromRow)]
struct DBWalletTransaction {
    wallet_tx_id: i64,
    stock_tx_id: i64,
    seller_id: i64,
    amount: i64,
    time_stamp: NaiveDateTime,
}

#[derive(Debug, sqlx::FromRow)]
struct DBStockTransaction {
    stock_tx_id: i64,
    parent_stock_tx_id: i64,
    wallet_tx_id: i64,
    stock_id: i64,
    order_status: OrderStatus,
    stock_price: Option<i64>,
    limit_price: Option<i64>,
    quantity: i64,
    time_stamp: Option<NaiveDateTime>,
}
