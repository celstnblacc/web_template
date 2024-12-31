use actix_cors::Cors;
use actix_web::{http::header, web, App, HttpServer, Responder, HttpResponse};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::collections::HashMap;
use std::fs;
use std::io::Write;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Task {
    id: u64,
    name: String,
    completed: bool
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct User {
    id: u64,
    username: String,
    password: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Database {
    tasks: HashMap<u64, Task>,
    users: HashMap<u64, User>
}

impl Database {
    fn new() -> Database { // like a constructor
        Database {
            tasks: HashMap::new(),
            users: HashMap::new()
        }
    }

    // CRUD data
    fn insert(&mut self,  task: Task) {
        self.tasks.insert(task.id, task);
    }

    fn get(&self, id: u64) -> Option<&Task> {
        self.tasks.get(&id)
    }

    fn get_all(&self) -> Vec<&Task> {
        self.tasks.values().collect()
    }

    fn delete(&mut self, id: &u64) {
        self.tasks.remove(&id);
    }

    fn update(&mut self, id: u64, task: Task) {
        self.tasks.insert(id, task);
    }

    // USER DATA RELATED FUNCTIONS
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

    //DATABASE SAVING 7:45
    // Convert haspmap to json
    // &self, is impl Database (Hashmap)
    fn save_to_file(&self) -> std::io::Result<()> { 
        let data = serde_json::to_string(&self)?; // MEANING: convert the struct to a string
        let mut file = fs::File::create("database.json")?;
        file.write_all(data.as_bytes())?;
        Ok(())
    }

    fn load_from_file() -> std::io::Result<Self> {
        match fs::read_to_string("database.json") {
            Ok(data) if !data.trim().is_empty() => {
                let database: Database = serde_json::from_str(&data)?;
                Ok(database)
            }
            Ok(_) | Err(_) => {
                // Return a new database if the file is empty or not found
                println!("Database file is empty or missing, initializing a new database.");
                Ok(Database::new())
            }
        }
    }
    
}

struct AppState { 
    database: Mutex<Database>
}

async fn create_task(state: web::Data<AppState>, task: web::Json<Task>) -> impl Responder {
    let mut database = state.database
    .lock()
    .unwrap(); // can replace by expect(msg: "Locked database")

    database.insert(task.into_inner()); // into_inner: get the  extract task and put it in the database
    let _ = database.save_to_file();
    HttpResponse::Ok().finish()
}

async fn read_tasks(state: web::Data<AppState>, id: web::Path<u64>) -> impl Responder {
    let mut database = state.database
    .lock()
    .unwrap(); // can replace by expect(msg: "Locked database")

    match database.get(id.into_inner()) { // match returns an Option
        Some(task) => HttpResponse::Ok().json(task),
        None => HttpResponse::NotFound().finish()
    }
}

async fn read_task(state: web::Data<AppState>, id: web::Path<u64>) -> impl Responder {
    let mut database = state.database
    .lock()
    .unwrap(); // can replace by expect(msg: "Locked database")
 
    match database.get(id.into_inner()) { // match returns an Option
        Some(task) => HttpResponse::Ok().json(task),
        None => HttpResponse::NotFound().finish()
    }
}

async fn read_all_tasks(state: web::Data<AppState>) -> impl Responder {
    let database = state.database
    .lock()
    .unwrap(); // can replace by expect(msg: "Locked database")

    HttpResponse::Ok().json(database.get_all())
}

async fn update_task(state: web::Data<AppState>, task: web::Json<Task>) -> impl Responder {
    let mut database = state.database
    .lock()
    .unwrap(); // can replace by expect(msg: "Locked database") 
    println!("Update database");

    database.update(task.id, task.clone());
    println!("Updated database");

    let _ = database.save_to_file();
    HttpResponse::Ok().finish() 
}

async fn delete_task(state: web::Data<AppState>, id: web::Path<u64>) -> impl Responder {
    let mut database = state.database
    .lock()
    .unwrap(); // can replace by expect(msg: "Locked database")     

    database.delete(&id.into_inner());
    let _ = database.save_to_file();
    HttpResponse::Ok().finish()
} 

async fn register_user(state: web::Data<AppState>, user: web::Json<User>) -> impl Responder {
    let mut database = state.database
    .lock()
    .unwrap(); // can replace by expect(msg: "Locked database")
    database.insert_user(user.into_inner());
    let _ = database.save_to_file();
    HttpResponse::Ok().finish()
}

async fn login_user(state: web::Data<AppState>, user: web::Json<User>) -> impl Responder {
    let database = state.database
    .lock()
    .unwrap(); // can replace by expect(msg: "Locked database")
    match database.get_user_by_name(&user.username) {
        Some(stored_user) if stored_user.password == user.password => {
            HttpResponse::Ok().body("Login successful")
        },
        _ => HttpResponse::Unauthorized().body("Login failed")
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let database = match Database::load_from_file() {
        Ok(database) => database,
        Err(e) => {
            println!("Error loading database: {}", e);
            Database::new()
        }
    };
    // Use AppState to store the locked database (mutex)
    let app_data = web::Data::new(AppState {
        database: Mutex::new(database) // can shared in multiple threads
    });

    // Create a new HTTP server
    HttpServer::new(move || {
        App::new() // Actix web
            .wrap(
                Cors::permissive()
                    .allowed_origin_fn(| origin, _req_head | {
                        origin.as_bytes().starts_with(b"http://localhost") || origin == "null"
                    })
                    .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                    .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
                    .allowed_header(header::CONTENT_TYPE)
                    .supports_credentials()
                    .max_age(3600)
            )
            // cloned? To allow multiple threads, dont worry it not cloning the database, it only clones the web::Data pointer
            .app_data(app_data.clone())
            .route("/task", web::post().to(create_task))
            .route("/tasks", web::get().to(read_all_tasks))
            .route("/task/{id}", web::get().to(read_task))
            .route("/task", web::put().to(update_task))    
            .route("/task/{id}", web::delete().to(delete_task))
            .route("/register", web::post().to(register_user))
            .route("/login", web::post().to(login_user))

    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await   
}