// SPDX-License-Identifier: MPL-2.0
use kanche::identity::{Provision, Store};
use std::io::{self, Read};
#[tokio::main]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => {
            println!("Operator operation completed");
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Operator operation failed: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
async fn run() -> Result<(), &'static str> {
    let url =
        std::env::var("KANCHE_DATABASE_URL").map_err(|_| "KANCHE_DATABASE_URL is required")?;
    let store = Store::new(&url)?;
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("migrate") => store.migrate().await,
        Some("provision-synthetic") => {
            let mut input = String::new();
            io::stdin()
                .take(32769)
                .read_to_string(&mut input)
                .map_err(|_| "Unable to read bounded stdin")?;
            if input.len() > 32768 {
                return Err("Input too large");
            };
            let p: Provision =
                serde_json::from_str(&input).map_err(|_| "Invalid provisioning JSON")?;
            store.provision(p).await
        }
        Some(
            op @ ("cleanup" | "revoke-all" | "remove-enrollment" | "disable-user" | "disable-app"
            | "remove-key"),
        ) => {
            store
                .maintenance(
                    op,
                    args.get(1).map_or("", String::as_str),
                    args.get(2).map_or("", String::as_str),
                )
                .await
        }
        _ => Err(
            "Use migrate, provision-synthetic, cleanup, revoke-all, remove-enrollment APP USER, disable-user UNUSED USER, disable-app APP, or remove-key SHA256_DIGEST",
        ),
    }
}
