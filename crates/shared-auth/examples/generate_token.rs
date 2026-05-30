use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize)]
struct Claims {
    sub: String,
    role: String,
    merchant_id: Option<String>,
    exp: usize,
}

const MERCHANT_ACTOR_ID: &str = "00000000-0000-0000-0000-000000000001";
const MERCHANT_ACCOUNT_ID: &str = "00000000-0000-0000-0000-000000000001";
const ADMIN_ACTOR_ID: &str = "00000000-0000-0000-0000-000000000002";

fn main() {
    let args: Vec<String> = env::args().collect();
    let role = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    let secret =
        env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret-change-in-production".to_string());

    let exp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize
        + 3600;

    match role {
        "merchant" => {
            let claims = Claims {
                sub: MERCHANT_ACTOR_ID.to_string(),
                role: "merchant".to_string(),
                merchant_id: Some(MERCHANT_ACCOUNT_ID.to_string()),
                exp,
            };
            let token = encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret(secret.as_bytes()),
            )
            .unwrap();
            println!("Merchant token:");
            println!("{token}");
            println!("\nClaims: sub={MERCHANT_ACTOR_ID}, role=merchant, merchant_id={MERCHANT_ACCOUNT_ID}");
        }
        "administrator" => {
            let claims = Claims {
                sub: ADMIN_ACTOR_ID.to_string(),
                role: "administrator".to_string(),
                merchant_id: None,
                exp,
            };
            let token = encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret(secret.as_bytes()),
            )
            .unwrap();
            println!("Administrator token:");
            println!("{token}");
            println!("\nClaims: sub={ADMIN_ACTOR_ID}, role=administrator");
        }
        _ => {
            eprintln!("Usage: cargo run -p shared-auth --example generate_token -- <merchant|administrator>");
            std::process::exit(1);
        }
    }
}
