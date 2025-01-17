use std::{fs, path::Path, process::Command};

use rusqlite::Connection;

fn main() {
    // Embed git hash
    {
        let output = Command::new("git")
            .args(["rev-parse", "--short=7", "HEAD"])
            .output();
        let git_hash = if let Ok(output) = output {
            String::from_utf8(output.stdout).unwrap()
        } else {
            String::from("")
        };
        println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    }

    // Setup DB to sqlx
    {
        let db_path = Path::new("./target/build.db").to_string_lossy().to_string();
        let _ = fs::remove_file(&db_path);

        let conn = Connection::open(&db_path).expect("Failed to open database");

        let sql = fs::read_to_string("./src/init.sql").expect("Failed to read init.sql");
        conn.execute_batch(&sql).expect("Failed to execute SQL");

        println!("cargo:rustc-env=DATABASE_URL=sqlite://target/build.db");
    }
}
