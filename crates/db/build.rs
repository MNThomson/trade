use std::{fs, path::Path};

use rusqlite::Connection;

fn main() {
    let db_path = Path::new("../../target/build.db")
        .to_string_lossy()
        .to_string();
    let _ = fs::remove_file(&db_path);

    let conn = Connection::open(&db_path).expect("Failed to open database");

    let sql = fs::read_to_string("./sql/init.sql").expect("Failed to read init.sql");
    conn.execute_batch(&sql).expect("Failed to execute SQL");

    println!("cargo:rustc-env=DATABASE_URL=sqlite://target/build.db");
}
