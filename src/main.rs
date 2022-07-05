use actix_web::{HttpServer, App, web, HttpResponse, Responder};
use tera::{Tera, Context}; 

async fn index(tera: web::Data<Tera>) -> impl Responder {
    let mut data = Context::new();
    data.insert("title", "CapitalCommander");
    data.insert("name","Johnny");

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
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
