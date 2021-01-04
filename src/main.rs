mod image_store;
mod hardware_control;

use actix_web::{HttpServer, web, HttpResponse, Responder, get, post, App};
use serde::{Serialize, Deserialize};
use std::sync::{Mutex, mpsc};
use std::ops::Deref;
use crate::image_store::ImageStore;
use std::process::exit;
use log::{error};
use crate::hardware_control::start_image_collection;
use std::sync::mpsc::Sender;

const ENDPOINT_AUFNAHME: &'static str = "aufnahme";

#[derive(Deserialize, Serialize)]
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
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();
    *app_state.shutdown_handle.lock().unwrap() = Some(shutdown_tx);
    start_image_collection(app_state, shutdown_rx, auftrag_json.0);
    HttpResponse::Ok()
}

#[get("/auftrag")]
async fn auftrag_get(data: web::Data<AppState>) -> impl Responder {
    let state = data.fortschritt.lock().unwrap();
    HttpResponse::Ok().json(state.deref())
}

#[get("/aufnahme")]
async fn aufnahme_get(progress: web::Data<AppState>) -> impl Responder {
    let progress = progress.image_store.get_image_list();
    let image_paths = progress.iter()
        .map(|image_name| format!("/{}/{}", ENDPOINT_AUFNAHME, image_name))
        .collect::<Vec<_>>();
    HttpResponse::Ok().json(image_paths)
}

#[get("/aufnahme/{name}")]
async fn aufnahem_single_get(image_name: web::Path<String>, app_state: web::Data<AppState>) -> impl Responder {
    match app_state.image_store.get_image(&image_name.0) {
        Ok(image) => {
            HttpResponse::Ok()
                .header("Content-Type", "image/jpeg")
                .body(image)
        }
        Err(err) => {
            match err {
                None => {HttpResponse::NotFound().finish()}
                Some(io_err) => {HttpResponse::InternalServerError().body(io_err.to_string())}
            }
        }
    }
}

pub struct AppState {
    fortschritt: Mutex<Fortschritt>,
    image_store: ImageStore,
    shutdown_handle: Mutex<Option<Sender<()>>>,
}

impl AppState {
    fn new() -> AppState {
        let image_store = match ImageStore::new() {
            Ok(image_store) => {image_store}
            Err(err) => {error!("{}", err); exit(1);}
        };

        AppState {
            fortschritt: Mutex::new(Fortschritt { runde: 0, aufnahme: 0 }),
            image_store,
            shutdown_handle: Mutex::new(None)
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // exit if faile to clear image folder

    let state = web::Data::new(AppState::new());
    HttpServer::new(
        move || App::new()
            .service(auftrag_get)
            .app_data(state.clone()))
        .bind("0.0.0.0:8000")?
        .run()
        .await
}