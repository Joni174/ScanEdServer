use tokio::sync::Mutex;
use std::collections::HashSet;
use tokio::io;
use std::iter::FromIterator;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::path::PathBuf;
use std::str::FromStr;

pub struct ImageStore {
    image_list: Mutex<HashSet<String>>,
}

impl ImageStore {
    pub async fn new() -> io::Result<ImageStore> {
        init_dir().await?;
        Ok(ImageStore { image_list: Mutex::new(HashSet::new()) })
    }

    pub async fn store_image(&self, name: String, image: &[u8]) -> io::Result<()> {
        let mut image_list = self.image_list.lock().await;
        save_image(&name, &image).await?;
        image_list.insert(name);
        Ok(())
    }

    pub async fn get_image_list(&self) -> Vec<String> {
        Vec::from_iter(self.image_list.lock().await.clone().into_iter())
    }

    pub async fn get_image(&self, name: &String) -> Result<Vec<u8>, Option<tokio::io::Error>> {
        let image_list = self.image_list.lock().await;
        if image_list.contains(name) {
            Ok(read_image(name).await.map_err(|err| Some(err))?)
        } else {
            Err(None)
        }
    }

    pub async fn reset(&self) -> io::Result<()> {
        self.image_list.lock().await.clear();
        init_dir().await
    }
}

async fn save_image(name: &str, img: &[u8]) -> Result<(), io::Error> {
    let mut file = File::create(image_folder().join(name)).await?;
    file.write_all(img).await?;
    Ok(())
}

async fn read_image(name: &str) -> tokio::io::Result<Vec<u8>> {
    let mut file = File::open(image_folder().join(name)).await?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).await?;
    Ok(buf)
}

async fn init_dir() -> tokio::io::Result<()> {
    if image_folder().exists() {
        tokio::fs::remove_dir_all(image_folder()).await?;
    }
    tokio::fs::create_dir_all(image_folder()).await?;
    Ok(())
}

fn image_folder() -> PathBuf {
    PathBuf::from_str("images").unwrap()
}