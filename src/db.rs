#[cfg(debug_assertions)]
use std::fs::remove_file;
use std::{fs::File, time::Duration};

use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

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
    pub async fn create_user(&self, user_name: String, password: String) -> Result<(), ()> {
        let _res = sqlx::query_as!(
            User,
            "INSERT INTO users (user_name, password) VALUES (?, ?)",
            user_name,
            password
        )
        .execute(&self.pool)
        .await
        .unwrap();

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_user(&self, user_name: String) -> Result<Option<()>, ()> {
        let row = sqlx::query_as!(User, "SELECT * FROM users WHERE user_name = ?", user_name)
            .fetch_one(&self.pool)
            .await
            .unwrap();

        println!("ASDSAD {:?}", row);
        Ok(None)
    }
}

#[derive(Debug)]
struct User {
    user_id: i64,
    user_name: String,
    password: String,
}
