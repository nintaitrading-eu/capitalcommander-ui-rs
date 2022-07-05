use actix_web::{HttpServer, App, web, HttpResponse, Responder};
use tera::{Tera, Context}; 
use serde::{Serialize, Deserialize};

#[derive(Debug, Deserialize)]
struct User {
    username: String,
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct LoginUser {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct TradingJournal {
    title: String,
    link: String,
    author: String,
}

#[derive(Debug, Deserialize)]
struct Entry {
    title: String,
    link: String,
}

async fn signup(tera: web::Data<Tera>) -> impl Responder {
    let mut data = Context::new();
    data.insert("title", "Sign Up");

    let rendered = tera.render("signup.html", &data).unwrap();
    HttpResponse::Ok().body(rendered)
}

async fn process_signup(data: web::Form<User>) -> impl Responder {
    println!("{:?}", data);
    HttpResponse::Ok().body(format!("Successfully saved user: {}", data.username))
}

async fn login(tera: web::Data<Tera>) -> impl Responder {
    let mut data = Context::new();
    data.insert("title", "Login");

    let rendered = tera.render("login.html", &data).unwrap();
    HttpResponse::Ok().body(rendered)
}

async fn process_login(data: web::Form<LoginUser>) -> impl Responder {
    println!("{:?}", data);
    HttpResponse::Ok().body(format!("Logged in: {}", data.username))
}

async fn entry(tera: web::Data<Tera>) -> impl Responder {
    let mut data = Context::new();
    data.insert("title", "Submit a journal entry");

    let rendered = tera.render("entry.html", &data).unwrap();
    HttpResponse::Ok().body(rendered)
}

async fn process_entry(data: web::Form<Entry>) -> impl Responder {
    println!("{:?}", data);
    HttpResponse::Ok().body(format!("Posted entry: {}", data.title))
}
async fn index(tera: web::Data<Tera>) -> impl Responder {
    let mut data = Context::new();

    let entries = [
        TradingJournal {
            title: String::from("A title"),
            link: String::from("A link here"),
            author: String::from("An author"),
        },
        TradingJournal {
            title: String::from("A title2"),
            link: String::from("A link2 here"),
            author: String::from("An author2"),
        },
    ];
    data.insert("title", "CapitalCommander");
    data.insert("entries", &entries);

    let rendered = tera.render("index.html", &data).unwrap();
    HttpResponse::Ok().body(rendered)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        let tera = Tera::new("templates/**/*").unwrap();
        App::new()
            .data(tera)
            .route("/", web::to(index))
            .route("/signup", web::to(signup))
            .route("/signup", web::get().to(signup))
            .route("/signup", web::post().to(process_signup))
            .route("/login", web::get().to(login))
            .route("/login", web::post().to(process_login))
            .route("/entry", web::get().to(entry))
            .route("/entry", web::post().to(process_entry))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
