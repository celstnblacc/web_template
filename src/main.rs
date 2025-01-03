/*
Foreign Exchange Rates API

Test endpoints with curl:
curl http://127.0.0.1:8080/forex
curl http://127.0.0.1:8080/forex/EURUSD

Update Forex price:
curl -X PUT -H "Content-Type: application/json" -d '{"symbol": "EURUSD", "price": 1.23}' http://127.0.0.1:8080/forex
 */

use actix_cors::Cors;
use actix_web::{http::header, web, App, HttpServer, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::collections::HashMap;
use reqwest::Client;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ForexPair {
    symbol: String,
    price: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ForexDatabase {
    forex_pairs: HashMap<String, ForexPair>,
}

impl ForexDatabase {
    fn new() -> ForexDatabase {
        ForexDatabase {
            forex_pairs: HashMap::new(),
        }
    }

    fn preload(&mut self) {
        self.forex_pairs.insert(
            "EURUSD".to_string(),
            ForexPair {
                symbol: "EURUSD".to_string(),
                price: 1.0,
            },
        );
        self.forex_pairs.insert(
            "USDJPY".to_string(),
            ForexPair {
                symbol: "USDJPY".to_string(),
                price: 1.0,
            },
        );
    }

    fn get(&self, symbol: &str) -> Option<&ForexPair> {
        self.forex_pairs.get(symbol)
    }

    fn get_all(&self) -> Vec<&ForexPair> {
        self.forex_pairs.values().collect()
    }

    fn update(&mut self, symbol: &str, price: f64) {
        if let Some(pair) = self.forex_pairs.get_mut(symbol) {
            pair.price = price;
        }
    }

    async fn fetch_latest_prices(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::new();
        let urls = vec![
            "https://api.exchangerate-api.com/v4/latest/USD",
            "https://api.exchangerate-api.com/v4/latest/EUR",
        ];

        for url in urls {
            let resp_text = client.get(url).send().await?.text().await?;
            let resp: serde_json::Value = serde_json::from_str(&resp_text)?;

            if let Some(rates) = resp.get("rates").and_then(|v| v.as_object()) {
                for (symbol, price) in rates {
                    if let Some(price) = price.as_f64() {
                        self.update(symbol, price);
                    }
                }
            } else {
                eprintln!("Rates not found in response from {}", url);
            }
        }

        Ok(())
    }
}

struct AppState {
    forex_data: Mutex<ForexDatabase>,
}

async fn get_forex_price(state: web::Data<AppState>, symbol: web::Path<String>) -> impl Responder {
    let forex_data = state.forex_data.lock().unwrap();
    match forex_data.get(&symbol.into_inner().to_uppercase()) {
        Some(pair) => HttpResponse::Ok().json(pair),
        None => HttpResponse::NotFound().finish(),
    }
}

async fn get_all_forex_prices(state: web::Data<AppState>) -> impl Responder {
    let forex_data = state.forex_data.lock().unwrap();
    HttpResponse::Ok().json(forex_data.get_all())
}

async fn update_forex_price(state: web::Data<AppState>, pair: web::Json<ForexPair>) -> impl Responder {
    let mut forex_data = state.forex_data.lock().unwrap();
    forex_data.update(&pair.symbol, pair.price);
    HttpResponse::Ok().finish()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let mut forex_data = ForexDatabase::new();
    forex_data.preload();
    if let Err(e) = forex_data.fetch_latest_prices().await {
        eprintln!("Failed to fetch latest Forex prices: {}", e);
    }

    let app_data = web::Data::new(AppState {
        forex_data: Mutex::new(forex_data),
    });

    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::permissive()
                    .allowed_origin_fn(|origin, _req_head| {
                        origin.as_bytes().starts_with(b"http://localhost") || origin == "null"
                    })
                    .allowed_methods(vec!["GET", "POST", "PUT"])
                    .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
                    .allowed_header(header::CONTENT_TYPE)
                    .supports_credentials()
                    .max_age(3600)
            )
            .app_data(app_data.clone())
            .route("/forex/{symbol}", web::get().to(get_forex_price))
            .route("/forex", web::get().to(get_all_forex_prices))
            .route("/forex", web::put().to(update_forex_price))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
