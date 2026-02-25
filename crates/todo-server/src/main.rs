mod broadcast;
mod db;
mod describe;
mod handlers;
mod models;

use actix_cors::Cors;
use actix_web::{web, App, HttpServer, middleware};
use clap::Parser;

#[derive(Parser)]
#[command(name = "todo-server")]
struct Args {
    /// Database file path
    #[arg(long, default_value = "./data/todos.db")]
    db: String,

    /// Listen address
    #[arg(long, default_value = "0.0.0.0:8080")]
    addr: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    println!("Initializing database at: {}", args.db);
    let pool = db::init_pool(&args.db);
    let broadcaster = broadcast::Broadcaster::new();

    // Determine frontend dist path
    let frontend_dir = std::env::var("FRONTEND_DIR")
        .unwrap_or_else(|_| "./frontend-dist".to_string());

    println!("Serving frontend from: {frontend_dir}");
    println!("Starting server at http://{}", args.addr);

    let frontend_dir_clone = frontend_dir.clone();
    let broadcaster_clone = broadcaster.clone();

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        let mut app = App::new()
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(broadcaster_clone.clone()))
            // API routes
            .route("/api/ws", web::get().to(handlers::ws_handler))
            .route("/api/describe", web::get().to(describe::describe))
            .route("/api/sections", web::get().to(handlers::get_sections))
            .route("/api/todos", web::get().to(handlers::list_todos))
            .route("/api/todos", web::post().to(handlers::create_todo))
            .route("/api/todos/{id}", web::get().to(handlers::get_todo))
            .route("/api/todos/{id}", web::patch().to(handlers::update_todo))
            .route("/api/todos/{id}", web::delete().to(handlers::delete_todo))
            .route("/api/todos/{id}/toggle", web::post().to(handlers::toggle_todo));

        // Serve frontend static files if the directory exists
        let fe_path = std::path::Path::new(&frontend_dir_clone);
        if fe_path.exists() {
            app = app.service(
                actix_files::Files::new("/", &frontend_dir_clone)
                    .index_file("index.html")
                    .default_handler(
                        actix_files::NamedFile::open(fe_path.join("index.html"))
                            .expect("frontend-dist/index.html not found"),
                    ),
            );
        }

        app
    })
    .bind(&args.addr)?
    .run()
    .await
}
