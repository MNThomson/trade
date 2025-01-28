#[cfg(debug_assertions)]
use std::fs::remove_file;
use std::{fs::File, time::Duration};

use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use tracing::error;

use crate::types::AppError;

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

        File::open(PATH).or_else(|_| File::create(PATH)).unwrap();
        let db = DB {
            pool: SqlitePoolOptions::new()
                .max_connections(50)
                .acquire_timeout(Duration::from_secs(3))
                .connect(format!("sqlite://{}", PATH).as_str())
                .await
                .unwrap(),
        };

        #[cfg(debug_assertions)]
        let _ = sqlx::query(INIT_SQL).execute(&db.pool).await;

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
}

#[derive(Debug)]
pub struct DbUser {
    pub user_id: i64,
    pub user_name: String,
    pub password: String,
    pub created_at: i64,
}
