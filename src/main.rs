mod image_store;
mod hardware_control;

use actix_web::{HttpServer, web, HttpResponse, Responder, get, post, App};
use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex};
use crate::image_store::ImageStore;
use std::process::exit;
use log::{error, info};
use crate::hardware_control::start_image_collection;
use env_logger::Env;
use tokio::task::JoinHandle;
use actix_web::web::Data;
use std::ops::Deref;

const ENDPOINT_AUFNAHME: &'static str = "aufnahme";

#[derive(Deserialize, Serialize, Debug)]
pub struct Auftrag {
    pub auftrag: Vec<i32>
}

#[derive(Serialize)]
pub struct Fortschritt {
    runde: i32,
    aufnahme: i32,
}

impl Fortschritt {
    pub fn set_aufnahme(&mut self, new_val: i32) {
        self.aufnahme = new_val;
    }
    pub fn set_runde(&mut self, new_val: i32) {
        self.runde = new_val;
    }
}

#[post("/auftrag")]
async fn auftrag_post(auftrag_json: web::Json<Auftrag>, app_state: web::Data<AppState>) -> impl Responder {
    info!("got auftrag: {:?}", &auftrag_json.0);
    let shutdown_handle = Arc::clone(&app_state.shutdown_handle);

    reset(&app_state, &shutdown_handle).await;

    start_image_collection(
        app_state,
        shutdown_handle,
        auftrag_json.0).await;
    HttpResponse::Ok()
}

async fn reset(app_state: &Data<AppState>, shutdown_handle: &Arc<Mutex<bool>>) {
    // shutdown previous image taking thread and wait for it
    *shutdown_handle.lock().unwrap() = true;
    if let Some(image_join_handle) = app_state.image_thread.lock().unwrap().take() {
        image_join_handle.await.unwrap_or_else(|_err| {error!("unable to reset image taking process as it has already stopped");});
    }
    *shutdown_handle.lock().unwrap() = false;
    *app_state.fortschritt.lock().unwrap() = Fortschritt{aufnahme: 0, runde: 0};
    app_state.image_store.lock().unwrap().reset().unwrap();
}

#[get("/auftrag")]
async fn auftrag_get(data: web::Data<AppState>) -> impl Responder {
    info!("serving auftrag status");
    let state = data.fortschritt.lock().unwrap();
    info!("serving auftrag status done");
    HttpResponse::Ok().json(state.deref())
}

#[get("/aufnahme")]
async fn aufnahme_get(progress: web::Data<AppState>) -> impl Responder {
    info!("serving aufnahmen index");
    let image_list = tokio::task::spawn_blocking(move || {
        let image_store = progress.image_store.lock().unwrap();
        image_store.get_image_list()
    }).await.unwrap();
    info!("serving aufnahmen index done");
    let image_paths = image_list.iter()
        .map(|image_name| format!("/{}/{}", ENDPOINT_AUFNAHME, image_name))
        .collect::<Vec<_>>();
    HttpResponse::Ok().json(image_paths)
}

#[get("/aufnahme/{name}")]
async fn aufnahme_single_get(image_name: web::Path<String>, app_state: web::Data<AppState>) -> impl Responder {
    let image_name2 = image_name.0.clone();
    info!("serving aufnahme: {}", image_name.0);
    let image = tokio::task::spawn_blocking(move || {
        let image_lock = app_state.image_store.lock().unwrap();
        image_lock.get_image(&image_name2)
    }).await.unwrap();
    match image {
        Ok(image) => {
            HttpResponse::Ok()
                .header("Content-Type", "image/jpeg")
                .header("Content-Length", image.len().to_string())
                .body(image)
        }
        Err(err) => {
            match err {
                None => { HttpResponse::NotFound().finish() }
                Some(io_err) => { HttpResponse::InternalServerError().body(io_err.to_string()) }
            }
        }
    }
}

pub struct AppState {
    fortschritt: Mutex<Fortschritt>,
    image_store: Mutex<ImageStore>,
    shutdown_handle: Arc<Mutex<bool>>,
    image_thread: Mutex<Option<JoinHandle<()>>>,
}

impl AppState {
    fn new() -> AppState {
        let image_store = match ImageStore::new() {
            Ok(image_store) => { image_store }
            Err(err) => {
                error!("{}", err);
                exit(1);
            }
        };

        AppState {
            fortschritt: Mutex::new(Fortschritt { runde: 0, aufnahme: 0 }),
            image_store: Mutex::new(image_store),
            shutdown_handle: Arc::new(Mutex::new(false)),
            image_thread: Mutex::new(None),
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // exit if faile to clear image folder
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    info!("Starting Server");

    let state = web::Data::new(AppState::new());
    HttpServer::new(
        move || App::new()
            .service(auftrag_get)
            .service(auftrag_post)
            .service(aufnahme_get)
            .service(aufnahme_single_get)
            .app_data(state.clone()))
        .bind("0.0.0.0:8000")?
        .workers(2)
        .run()
        .await
}
