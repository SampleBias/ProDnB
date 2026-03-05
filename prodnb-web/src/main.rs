//! ProDnB Web Server
//!
//! A web interface for converting PDB protein files to Strudel music code.
//! Upload a PDB file and get aesthetically pleasing Drum & Bass Strudel code.

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use askama::Template;

mod handlers;
mod templates;

use handlers::{upload_pdb, protein_function, infer_beat_design, generate_orchestration_instruction, generate_strudel, generate_strudel_stream, map_primitives, assemble_from_primitives, health_check, api_info};
use templates::IndexTemplate;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init();

    let bind_address = std::env::var("BIND_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string());

    let static_dir = std::path::PathBuf::from("./prodnb-web/static");

    log::info!("Starting ProDnB Web Server on {}", bind_address);
    log::info!("Access at http://{}", bind_address);

    HttpServer::new(move || {
        let static_dir = static_dir.clone();
        App::new()
            .route("/", web::get().to(index))
            .route("/health", web::get().to(health_check))
            .service(
                web::scope("/api")
                    .route("", web::get().to(api_info))
                    .route("/upload", web::post().to(upload_pdb))
                    .route("/protein-function", web::post().to(protein_function))
                    .route("/infer-beat-design", web::post().to(infer_beat_design))
                    .route("/generate-orchestration-instruction", web::post().to(generate_orchestration_instruction))
                    .route("/map", web::post().to(map_primitives))
                    .route("/assemble", web::post().to(assemble_from_primitives))
                    .route("/generate", web::post().to(generate_strudel))
                    .route("/generate/stream", web::post().to(generate_strudel_stream))
            )
            .service(actix_files::Files::new("/static", static_dir).show_files_listing())
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
