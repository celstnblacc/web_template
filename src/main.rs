```rust
use actix_cors::Cors;
use actix_web::{http::header, web, App, HttpServer, Responder, HttpResponse};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::collections::HashMap;
use std::fs;
use std::io::Write;

use reqwest::Client as HttpClient;
use async_trait::async_trait;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct FitnessProgress {
    id: u64,
    user_id: u64,
    date: String,
    timezone: String,
    steps: u32,
    calories_burned: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct User {
    id: u64,
    username: String,
    password: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Database {
    fitness_progress: HashMap<u64, FitnessProgress>,
    users: HashMap<u64, User>
}

impl Database {
    fn new() -> Database {
        Database {
            fitness_progress: HashMap::new(),
            users: HashMap::new()
        }
    }

    fn insert_progress(&mut self, progress: FitnessProgress) {
        self.fitness_progress.insert(progress.id, progress);
    }

    fn get_progress(&self, id: u64) -> Option<&FitnessProgress> {
        self.fitness_progress.get(&id)
    }

    fn get_all_progress(&self) -> Vec<&FitnessProgress> {
        self.fitness_progress.values().collect()
    }

    fn delete_progress(&mut self, id: &u64) {
        self.fitness_progress.remove(&id);
    }

    fn update_progress(&mut self, id: u64, progress: FitnessProgress) {
        self.fitness_progress.insert(id, progress);
    }

    fn insert_user(&mut self, user: User) {
        self.users.insert(user.id, user);
    }

    fn get_user_by_name(&self, username: &str) -> Option<&User> {
        self.users.values().find(|user| user.username == username)
    }

    fn get_user_by_id(&self, id: u64) -> Option<&User> {
        self.users.get(&id)
    }

    fn delete_user_by_id(&mut self, id: &u64) {
        self.users.remove(&id);
    }

    fn update_user_by_id(&mut self, id: u64, user: User) {
        self.users.insert(id, user);
    }

    fn save_to_file(&self) -> std::io::Result<()> {
        let data = serde_json::to_string(&self)?;
        let mut file = fs::File::create("database.json")?;
        file.write_all(data.as_bytes())?;
        Ok(())
    }

    fn load_from_file() -> std::io::Result<Self> {
        let data = fs::read_to_string("database.json")?;
        let database: Database = serde_json::from_str(&data)?;
        Ok(database)
    }
}

struct AppState {
    database: Mutex<Database>
}

async fn create_progress(state: web::Data<AppState>, progress: web::Json<FitnessProgress>) -> impl Responder {
    let mut database = state.database.lock().unwrap();
    database.insert_progress(progress.into_inner());
    let _ = database.save_to_file();
    HttpResponse::Ok().finish()
}

async fn read_progress_by_id(state: web::Data<AppState>, id: web::Path<u64>) -> impl Responder {
    let database = state.database.lock().unwrap();
    match database.get_progress(id.into_inner()) {
        Some(progress) => HttpResponse::Ok().json(progress),
        None => HttpResponse::NotFound().finish()
    }
}

async fn read_all_progress(state: web::Data<AppState>) -> impl Responder {
    let database = state.database.lock().unwrap();
    HttpResponse::Ok().json(database.get_all_progress())
}

async fn update_progress(state: web::Data<AppState>, progress: web::Json<FitnessProgress>) -> impl Responder {
    let mut database = state.database.lock().unwrap();
    database.update_progress(progress.id, progress.clone());
    let _ = database.save_to_file();
    HttpResponse::Ok().finish()
}

async fn delete_progress(state: web::Data<AppState>, id: web::Path<u64>) -> impl Responder {
    let mut database = state.database.lock().unwrap();
    database.delete_progress(&id.into_inner());
    let _ = database.save_to_file();
    HttpResponse::Ok().finish()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let database = match Database::load_from_file() {
        Ok(database) => database,
        Err(_) => Database::new(),
    };

    let app_data = web::Data::new(AppState {
        database: Mutex::new(database)
    });

    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::permissive()
                    .allowed_origin_fn(|origin, _req_head| {
                        origin.as_bytes().starts_with(b"http://localhost") || origin == "null"
                    })
                    .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                    .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
                    .allowed_header(header::CONTENT_TYPE)
                    .supports_credentials()
                    .max_age(3600)
            )
            .app_data(app_data.clone())
            .route("/progress", web::post().to(create_progress))
            .route("/progresses", web::get().to(read_all_progress))
            .route("/progress/{id}", web::get().to(read_progress_by_id))
            .route("/progress", web::put().to(update_progress))
            .route("/progress/{id}", web::delete().to(delete_progress))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
```