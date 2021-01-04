use std::sync::Mutex;
use std::collections::HashSet;
use std::iter::FromIterator;
use std::path::PathBuf;
use std::str::FromStr;
use std::fs::File;
use std::io;
use std::io::{Write, Read};
use std::ops::Deref;

pub struct ImageStore {
    image_list: Mutex<HashSet<String>>,
}

impl ImageStore {
    pub fn new() -> io::Result<ImageStore> {
        init_dir()?;
        Ok(ImageStore { image_list: Mutex::new(HashSet::new()) })
    }

    pub fn store_image(&self, name: String, image: &[u8]) -> io::Result<()> {
        let mut image_list = self.image_list.lock().unwrap();
        save_image(&name, image)?;
        image_list.insert(name);
        Ok(())
    }

    pub fn get_image_list(&self) -> Vec<String> {
        let image_list = self.image_list.lock().unwrap();
        Vec::from_iter(image_list.deref().clone().into_iter())
    }

    pub fn get_image(&self, name: &String) -> Result<Vec<u8>, Option<io::Error>> {
        let image_list = self.image_list.lock().unwrap();
        if image_list.contains(name) {
            Ok(read_image(name).map_err(|err| Some(err))?)
        } else {
            Err(None)
        }
    }

    pub fn reset(&self) -> io::Result<()> {
        let mut image_list = self.image_list.lock().unwrap();
        image_list.clear();
        init_dir()
    }
}

fn save_image(name: &str, img: &[u8]) -> Result<(), io::Error> {
    let mut file = File::create(image_folder().join(name))?;
    file.write_all(img)?;
    Ok(())
}

fn read_image(name: &str) -> io::Result<Vec<u8>> {
    let mut file = File::open(image_folder().join(name))?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

fn init_dir() -> io::Result<()> {
    if image_folder().exists() {
        std::fs::remove_dir_all(image_folder())?;
    }
    std::fs::create_dir(image_folder())?;
    Ok(())
}

fn image_folder() -> PathBuf {
    PathBuf::from_str("images").unwrap()
}