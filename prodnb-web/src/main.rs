//! ProDnB Web Server
//!
//! A web interface for converting PDB protein files to Strudel music code.
//! Upload a PDB file and get aesthetically pleasing Drum & Bass Strudel code.

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use askama::Template;

mod handlers;
mod templates;

use handlers::{upload_pdb, generate_strudel, health_check};
use templates::IndexTemplate;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init();

    let bind_address = std::env::var("BIND_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string());

    // Get the static files directory path
    let static_dir = std::path::PathBuf::from("./prodnb-web/static");
    
    log::info!("Starting ProDnB Web Server on {}", bind_address);
    log::info!("Access at http://{}", bind_address);
    log::info!("Static files from: {}", static_dir.display());

    HttpServer::new(move || {
        let static_dir = static_dir.clone();
        App::new()
            // Static files
            .service(actix_files::Files::new("/static", static_dir).show_files_listing())
            // Routes
            .route("/", web::get().to(index))
            .route("/health", web::get().to(health_check))
            .route("/api/upload", web::post().to(upload_pdb))
            .route("/api/generate", web::post().to(generate_strudel))
    })
    .bind(&bind_address)?
    .run()
    .await
}

async fn index() -> impl Responder {
    let template = IndexTemplate {
        title: "ProDnB - PDB to Strudel Converter".to_string(),
    };
    
    match template.render() {
        Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
        Err(e) => {
            log::error!("Template render error: {}", e);
            HttpResponse::InternalServerError().body("Template error")
        }
    }
}
