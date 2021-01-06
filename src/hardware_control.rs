use crate::{AppState, Auftrag};
use std::sync::{Arc, Mutex};
use std::thread;
use actix_web::web::Data;
use crate::hardware_control::camera::{start_camera, get_camera_stream, take_image};
use crate::hardware_control::image_communication::{store_new_image, update_status};
use crate::hardware_control::motor::{calculate_steps, move_steps};
use std::time::Duration;
use log::{info};
use image::{RgbImage, EncodableLayout};
use std::ops::Deref;
use std::thread::JoinHandle;

pub fn start_image_collection(progress: actix_web::web::Data<AppState>,
                              shutdown_rx: Arc<Mutex<bool>>,
                              auftrag: Auftrag) -> JoinHandle<()> {
    thread::spawn(move || hardware_control_loop(&progress, shutdown_rx, auftrag))
}

fn hardware_control_loop(progress: &Data<AppState>, shutdown_rx: Arc<Mutex<bool>>, auftrag: Auftrag) {
    // let camera = start_camera();
    // let mut stream = get_camera_stream(camera);
    for (round, images_this_round) in auftrag.auftrag.iter().enumerate() {
        for image_nr in 0..*images_this_round {
            info!("taking image");

            // let image = take_image(&mut stream);
            let image = new_random_image();

            let shutdown_flag = shutdown_rx.lock().unwrap();
            if *shutdown_flag.deref() {
                info!("got shutdown message");
                return;
            } else {
                store_new_image(&progress.image_store, round, image_nr, &image);
            }
            drop(shutdown_flag);

            let steps = calculate_steps(*images_this_round as u32);

            thread::sleep(Duration::from_secs(1));
            move_steps(steps);
            update_status(&progress.fortschritt, round as i32, image_nr);
        }
    }
    info!("Finished taking images")
}

mod camera {
    use eye::traits::{ImageStream, Device};
    use eye::image::CowImage;
    use eye::prelude::Context;
    use eye::format::FourCC;

    pub fn take_image<'a>(stream: &'a mut Box<ImageStream>) -> CowImage<'a> {
        let image = stream.next()
            .expect("Camera stream is dead")
            .expect("Failed to capture frame");
        image
    }

    pub fn get_camera_stream<'a>(dev: Box<dyn Device + Send>) -> Box<ImageStream<'a>> {
        let stream = dev.stream().expect("Failed to setup capture stream");
        stream
    }

    pub fn start_camera() -> Box<dyn Device + Send> {
        let devices = Context::enumerate_devices();
        let mut dev = Context::open_device(&devices[0]).expect("Failed to open video device");
        let format = dev.format().expect("Unable to get Format from Camera");
        dev.set_format(&eye::format::Format::new(
            format.width,
            format.height,
            eye::format::PixelFormat::Custom(FourCC::new(b"JPEG"))))
            .expect("Unable to set Pixel Format to JPEG");
        dev
    }
}

mod image_communication {
    use crate::image_store::ImageStore;
    use crate::Fortschritt;
    use std::sync::Mutex;

    pub fn update_status(fortschritt: &Mutex<Fortschritt>, current_round: i32, current_image: i32) {
        let mut fortschritt = fortschritt.lock().unwrap();
        fortschritt.set_aufnahme(current_image + 1);
        fortschritt.set_runde(current_round + 1);
    }

    pub fn store_new_image(image_store: &ImageStore, round: usize, image_nr: i32, image: &Vec<u8>) {
        image_store.store_image(
            format!("{}_{}.jpg", round, image_nr),
            &image).expect("Error storing image");
    }
}

mod motor {
    pub fn move_steps(pulses: u32) {
        for _ in 0..pulses {
            // motor on
            // delay
            // motor off
            // delay
        }
    }

    pub fn calculate_steps(_images_for_round: u32) -> u32 {
        1
    }
}

fn new_random_image() -> Vec<u8> {
    let width : u32 = 300;
    let height: u32 = 300;
    let mut image: RgbImage = image::RgbImage::new(width, height);
    let c1 = rand::random::<u8>();
    let c2 = rand::random::<u8>();
    let c3 = rand::random::<u8>();
    for x in 0..width {
        for y in 0..height {
            image.put_pixel(x, y, image::Rgb([c1, c2, c3]));
        }
    }
    let mut vec = Vec::<u8>::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new(&mut vec);
    encoder.encode(image.as_bytes(), width, height, image::ColorType::Rgb8)
        .expect("unable to encode test image");
    vec
}