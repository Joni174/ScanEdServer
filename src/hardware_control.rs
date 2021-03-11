use crate::{AppState, Auftrag};
use std::sync::{Arc, Mutex};
use std::thread;
use crate::hardware_control::camera::{start_camera};
use crate::hardware_control::image_communication::{store_new_image, update_status};
use std::time::Duration;
use log::{info, warn};
use std::ops::Deref;
use tokio::task::JoinHandle;
use rust_gpiozero::*;
use crate::image_store::ImageStore;
use rascam::SimpleCamera;


pub async fn start_image_collection(progress: actix_web::web::Data<AppState>,
                              shutdown_rx: Arc<Mutex<bool>>,
                              auftrag: Auftrag) -> JoinHandle<()> {
    tokio::task::spawn_blocking(move || motor_movement(progress, shutdown_rx, auftrag.auftrag))
}

fn motor_movement(progress: actix_web::web::Data<AppState>,
                  shutdown_rx: Arc<Mutex<bool>>,
                  runden:Vec<i32>) {
    let ms1 = LED::new(4);
    let ms2 = LED::new(3);
    let ms3 = LED::new(2);
    let dir = LED::new(14);
    let step = LED::new(15);

    let led_user = LED::new(17);
    let led_error = LED::new(22);
    let led_cam = LED::new(27);
    let mut button = Button::new(23);


    ms1.on();
    ms2.on();
    ms3.on();
    dir.on();

    led_error.on();
    led_user.on();
    led_cam.on();
    thread::sleep(Duration::from_secs(4));
    let mut camera = start_camera();
    led_error.off();
    led_user.off();
    led_cam.off();

    for (round, number_images) in runden.iter().enumerate() {

        // user input button must be pressed to start round
        info!("wait for user input");
        wait_for_button_press(&led_user, &mut button);

        for image_nr in 0..*number_images {
            motor::move_pulses(&step, &number_images);

            handle_new_image(&progress.image_store, &led_cam, &mut camera, round, image_nr);

            if shutdown_message_arrived(&shutdown_rx) {
                return;
            }
            update_status(&progress.fortschritt, round as i32, image_nr);
        }
        info!("round finished")
    }
}

fn handle_new_image<'a>(image_store: &Mutex<ImageStore>,
                        led_cam: &LED,
                        camera: &mut SimpleCamera,
                        round: usize, image_nr: i32) {
    led_cam.on();
    info!("taking image round: {}, image: {}", round, image_nr);
    let image = camera.take_one().expect("unable to take image with camera");
    store_new_image(&image_store, round, image_nr, &image);
    led_cam.off();
}

fn shutdown_message_arrived(shutdown_rx: &Arc<Mutex<bool>>) -> bool {
    let shutdown_flag = shutdown_rx.lock().unwrap();
    if *shutdown_flag.deref() {
        warn!("got shutdown message");
        true
    } else {
        false
    }
}

fn wait_for_button_press(led_user: &LED, button: &mut Button) {
    led_user.on();
    button.wait_for_press(None);
    led_user.off();
}

mod motor {
    use rust_gpiozero::LED;
    use std::thread;
    use std::time::Duration;



    const STEPS_FOR_FULL_ROTATION: i32 = 3200;
    const MICROS_PULSE: u64 = 100;
    const MICROS_SPEED_CONTROL: u64 = 4000;


    pub fn move_pulses(step: &LED, number_images: &i32) {
        let pulses: usize = STEPS_FOR_FULL_ROTATION as usize / *number_images as usize;
        for _ in 0..pulses {
            step.on();
            thread::sleep(Duration::from_micros(MICROS_PULSE));
            step.off();
            thread::sleep(Duration::from_micros(MICROS_SPEED_CONTROL));
        }
    }
}

mod camera {
    use std::{time, thread};
    use rascam::*;

    pub fn start_camera() -> SimpleCamera {
        let info = info().unwrap();
        let mut camera = SimpleCamera::new(info.cameras[0].clone()).unwrap();
        camera.activate().unwrap();

        let sleep_duration = time::Duration::from_millis(2000);
        thread::sleep(sleep_duration);
        camera.take_one().expect("unable to take first initialization image");
        camera.take_one().expect("unable to take second initialization image");
        camera.take_one().expect("unable to take third initialization image");

        camera
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

    pub fn store_new_image(image_store: &Mutex<ImageStore>, round: usize, image_nr: i32, image: &Vec<u8>) {
        image_store.lock().unwrap().store_image(
            format!("{}_{}.jpg", round, image_nr),
            &image
        ).expect("Error storing image");
    }
}


// fn new_random_image() -> Vec<u8> {
//     let width : u32 = 300;
//     let height: u32 = 300;
//     let mut image: RgbImage = image::RgbImage::new(width, height);
//     let c1 = rand::random::<u8>();
//     let c2 = rand::random::<u8>();
//     let c3 = rand::random::<u8>();
//     for x in 0..width {
//         for y in 0..height {
//             image.put_pixel(x, y, image::Rgb([c1, c2, c3]));
//         }
//     }
//     let mut vec = Vec::<u8>::new();
//     let mut encoder = image::codecs::jpeg::JpegEncoder::new(&mut vec);
//     encoder.encode(image.as_bytes(), width, height, image::ColorType::Rgb8)
//         .expect("unable to encode test image");
//     vec
// }

// fn new_random_image(image_path: PathBuf) -> Vec<u8> {
//     std::fs::read(image_path.to_path_buf()).unwrap()
// }

// const TEST_IMAGE_FOLDER: &'static str = "test_images";
