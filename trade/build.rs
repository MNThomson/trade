use std::process::Command;

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
}
