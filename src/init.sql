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
    password TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') || substr(strftime('%f', 'now'), 4))
) STRICT;

CREATE TABLE stocks (
    stock_id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    stock_name TEXT NOT NULL
) STRICT;

-- admin/pass
INSERT INTO users (user_id, user_name, password) VALUES (1, "admin", "$argon2id$v=19$m=1024,t=1,p=1$HAZcjX8wBnPhvVhYBpXO5g$H009UoKExbLzSHbl5Ru6WEQ4djyRi5sU8fkfCwk8ulI");
