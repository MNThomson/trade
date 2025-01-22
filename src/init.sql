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
    user_id TEXT NOT NULL PRIMARY KEY,
    user_name TEXT NOT NULL UNIQUE,
    password TEXT NOT NULL
) STRICT;

CREATE TABLE stocks (
    stock_id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    stock_name TEXT NOT NULL
) STRICT;

INSERT INTO users (user_id, user_name, password) VALUES ("01D39ZY06FGSCTVN4T2V9PKHFZ", "admin", "pass");
