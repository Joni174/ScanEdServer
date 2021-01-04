use crate::{AppState, Auftrag};
use std::sync::mpsc;
use std::thread;
use std::sync::mpsc::Receiver;
use actix_web::web::Data;
use crate::hardware_control::camera::{start_camera, get_camera_stream, take_image};
use crate::hardware_control::image_communication::{reset, store_new_image, update_status};
use crate::hardware_control::motor::{calculate_steps, move_steps};

pub fn start_image_collection(progress: actix_web::web::Data<AppState>,
                              shutdown_rx: mpsc::Receiver<()>,
                              auftrag: Auftrag) {
    thread::spawn(move || hardware_control_loop(&progress, shutdown_rx, auftrag));
}

fn hardware_control_loop(progress: &Data<AppState>, shutdown_rx: Receiver<()>, auftrag: Auftrag) {
    let camera = start_camera();
    let mut stream = get_camera_stream(camera);
    for (round, images_this_round) in auftrag.auftrag.iter().enumerate() {
        for image_nr in 0..*images_this_round {
            if shutdown_rx.try_recv() != Ok(()) {
                reset(&progress.image_store);
                return;
            }

            let image = take_image(&mut stream);

            store_new_image(&progress.image_store, round, image_nr, &image);
            update_status(&progress.fortschritt, round, image_nr);

            let steps = calculate_steps(*images_this_round as u32);
            move_steps(steps);
        }
    }
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
    use eye::image::CowImage;

    pub fn update_status(fortschritt: &Mutex<Fortschritt>, current_round: usize, current_image: i32) {
        let mut fortschritt = fortschritt.lock().unwrap();
        fortschritt.set_aufnahme(current_image);
        fortschritt.set_runde(current_round as i32);
    }

    pub fn store_new_image(image_store: &ImageStore, round: usize, image_nr: i32, image: &CowImage) {
        image_store.store_image(
            format!("{}_{}.jpg", round, image_nr),
            &image.as_bytes()).expect("Error storing image");
    }

    pub fn reset(image_store: &ImageStore) {
        image_store.reset().expect("Error resetting image store")
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