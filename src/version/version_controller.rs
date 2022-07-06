use actix_web::{web, HttpResponse, Responder};
use tera::{Tera, Context}; 
use chrono;

use crate::version::version_dto;

pub async fn index(tera: web::Data<Tera>) -> impl Responder {
    let mut data = Context::new();
    let records = [
        version_dto::VersionDto {
            version_id: 69,
            database_version: String::from("0.1.0"),
            database_version_info: String::from("testinfo database"),
            application_version: String::from("0.1.0"),
            application_version_info: String::from("testinfo application"),
            date_created: chrono::offset::Local::now().naive_local(),
            date_modified: chrono::offset::Local::now().naive_local(),
        },
    ];
    data.insert("title", "Version");
    data.insert("records", &records);

    let rendered = tera.render("version.html", &data).unwrap();
    HttpResponse::Ok().body(rendered)
}
