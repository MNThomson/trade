-- https://briandouglas.ie/sqlite-defaults/
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA busy_timeout = 5000;
PRAGMA cache_size = -20000; -- 20MB
PRAGMA foreign_keys = ON;
PRAGMA auto_vacuum = INCREMENTAL;
PRAGMA temp_store = MEMORY;
PRAGMA mmap_size = 2147483648; -- 2GB
PRAGMA page_size = 8192; -- 8Kb

CREATE TABLE users (
    user_id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    user_name TEXT NOT NULL UNIQUE,
    password TEXT NOT NULL
    --created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
) STRICT;

CREATE TABLE user_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    token TEXT NOT NULL UNIQUE,
    FOREIGN KEY (user_id) REFERENCES users(id)
) STRICT;
