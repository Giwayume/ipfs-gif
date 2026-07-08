use std::{
    io::{ self, Cursor },
    env,
    error::Error,
    path::PathBuf,
};
use axum::body::Bytes;
use image::{
    ImageReader, ImageFormat,
    AnimationDecoder, ImageDecoder,
    codecs::gif::GifDecoder,
    codecs::png::PngDecoder,
    codecs::webp::WebPDecoder,
};
use reqwest::multipart::{ Form, Part };
use serde::Deserialize;
use tokio::time::{ interval, Duration };
use uuid::Uuid;

use crate::database;
use crate::util::secrets::{ secrets_config };

pub static TEMPORARY_IMAGE_DIRECTORY: &str = "uploads/assets/images/tmp";
pub static QUARANTINE_IMAGE_DIRECTORY: &str = "uploads/assets/images/quarantine";

#[derive(Clone, Debug)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub size: u32,
    pub frames: u32,
}

#[allow(unused)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct IpfsAddResponse {
    bytes: Option<i64>,
    hash: Option<String>,
    mode: Option<String>,
    mtime: Option<i64>,
    mtime_nsecs: Option<i64>,
    name: Option<String>,
    size: Option<String>,
}

/**
 * Returns the path to the temporary storage folder where images are uploaded to.
 */
async fn get_temporary_storage_path() -> PathBuf {
    let path = if cfg!(debug_assertions) {
        env::current_dir().unwrap().join(TEMPORARY_IMAGE_DIRECTORY)
    } else {
        env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .join(TEMPORARY_IMAGE_DIRECTORY)
    };

    if !path.exists() {
        tokio::fs::create_dir_all(&path).await.unwrap();
    }

    path
}

/**
 * Returns the base path to the images quarantine folder where images are moved to
 * after uploading in order to scan for NSFW content.
 */
async fn get_quarantine_storage_path() -> PathBuf {
    let path = if cfg!(debug_assertions) {
        env::current_dir().unwrap().join(QUARANTINE_IMAGE_DIRECTORY)
    } else {
        env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .join(QUARANTINE_IMAGE_DIRECTORY)
    };

    if !path.exists() {
        tokio::fs::create_dir_all(&path).await.unwrap();
    }

    path
}

/**
 * Stores the images defined by the given bytes in temporary storage and returns the filename.
 */
pub async fn store_temporary_image(content_type: String, data: Bytes) -> Result<String, Box<dyn Error>> {
    let mut file_extension = match content_type.as_str() {
        "image/apng" => Some("apng"),
        // "image/avif" => Some("avif"),
        "image/gif" => Some("gif"),
        "image/png" => Some("png"),
        "image/webp" => Some("webp"),
        _ => None,
    };

    if file_extension.is_none() {
        return Err(
            Box::new(
                io::Error::new(io::ErrorKind::Other, "Unsupported file type uploaded.")
            )
        );
    }

    // Validate that the file is actually an image.
    let reader = ImageReader::new(Cursor::new(&data)).with_guessed_format()?;
    let format: ImageFormat = reader.format().ok_or("Unknown image format")?;
    let _ = reader.decode()?;

    file_extension = match format {
        // ImageFormat::Avif => Some("avif"),
        ImageFormat::Gif => Some("gif"),
        ImageFormat::Png => Some("png"),
        ImageFormat::WebP => Some("webp"),
        _ => file_extension,
    };

    // Store the file in a temporary directory.
    let file_name = format!("{}.{}", Uuid::new_v4(), file_extension.unwrap());
    let storage_path = get_temporary_storage_path().await;
    let path = storage_path.join(&file_name);
    if let Err(error) = tokio::fs::write(&path, &data).await {
        tracing::warn!("Error storing a temporary file upload. {:?}", error);
        return Err(Box::new(error));
    }

    Ok(file_name)
}

/**
 * Retrieve the bytes of the temporary image.
 */
#[allow(unused)]
pub async fn get_temporary_image(
    temporary_image_filename: &str,
) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    let temporary_storage_path = get_temporary_storage_path().await;
    let temporary_image_path = temporary_storage_path.join(temporary_image_filename);

    if !temporary_image_path.exists() {
        return Err(
            Box::new(
                io::Error::new(io::ErrorKind::Other, "Uploaded image not found.")
            )
        );
    }

    let bytes = match tokio::fs::read(&temporary_image_path).await {
        Ok(bytes) => bytes,
        Err(err) => {
            tracing::warn!("Failed to read bytes from temporary image {:?}", err);
            return Err(Box::new(err));
        }
    };

    Ok(bytes)
}

/**
 * Use ffprobe to determine metadata for the temporary image.
 */
pub async fn get_temporary_image_info(
    temporary_image_filename: &str,
) -> Result<ImageInfo, Box<dyn Error + Send + Sync>> {
    let temporary_storage_path = get_temporary_storage_path().await;
    let temporary_image_path = temporary_storage_path.join(temporary_image_filename);

    if !temporary_image_path.exists() {
        return Err(
            Box::new(
                io::Error::new(io::ErrorKind::Other, "Uploaded image not found.")
            )
        );
    }

    let (mut width, mut height, size, mut frames) = match ffprobe::ffprobe(&temporary_image_path) {
        Ok(info) => {
            let stream = match info.streams.first() {
                Some(stream) => stream,
                None => return Err(Box::new(
                    io::Error::new(io::ErrorKind::Other, "No streams.")
                )),
            };

            let width = u32::try_from(stream.width.unwrap_or(1)).unwrap_or(1);
            let height = u32::try_from(stream.height.unwrap_or(1)).unwrap_or(1);
            let frames = stream.nb_frames.clone()
                .unwrap_or_else(|| String::from("0"))
                .parse::<u32>()
                .unwrap_or(1);

            let size = info.format.size
                .parse::<u32>()
                .unwrap_or(1);

            (width, height, size, frames)
        },
        Err(err) => {
            return Err(Box::new(err));
        }
    };

    // This means ffprobe failed to find info about the image, fall back to the slow method
    if width == 0 || height == 0 || frames == 0 {
        let bytes = match tokio::fs::read(&temporary_image_path).await {
            Ok(bytes) => bytes,
            Err(err) => {
                tracing::warn!("Failed to read bytes from temporary image {:?}", err);
                return Err(Box::new(err));
            }
        };

        let reader = ImageReader::new(Cursor::new(&bytes)).with_guessed_format()?;
        let format = reader.format().ok_or_else(|| std::io::Error::new(
            std::io::ErrorKind::Other, "Unknown image format"
        ))?;

        (width, height, frames) = match format {
            ImageFormat::Gif => {
                let decoder = GifDecoder::new(Cursor::new(&bytes))?;
                let (width, height) = decoder.dimensions();
                let frames = u32::try_from(decoder.into_frames().take(2).count()).unwrap_or(1);
                (width, height, frames)
            },
            ImageFormat::Png => {
                let decoder = PngDecoder::new(Cursor::new(&bytes))?;
                let (width, height) = decoder.dimensions();
                let frames = u32::try_from(decoder.apng()?.into_frames().take(2).count()).unwrap_or(1);
                (width, height, frames)
            },
            ImageFormat::WebP => {
                let decoder = WebPDecoder::new(Cursor::new(&bytes))?;
                let (width, height) = decoder.dimensions();
                let frames = u32::try_from(decoder.into_frames().take(2).count()).unwrap_or(1);
                (width, height, frames)
            },
            _ => return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Image format not supported.",
            ))),
        };
    }

    Ok(ImageInfo {
        width,
        height,
        size,
        frames,
    })
}

/**
 * Retrieve the bytes of the temporary image.
 */
pub async fn get_quarantine_image(
    quarantine_image_filename: &str,
) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    let quarantine_storage_path = get_quarantine_storage_path().await;
    let quarantine_image_path = quarantine_storage_path.join(quarantine_image_filename);

    if !quarantine_image_path.exists() {
        return Err(
            Box::new(
                io::Error::new(io::ErrorKind::Other, "Uploaded image not found.")
            )
        );
    }

    let bytes = match tokio::fs::read(&quarantine_image_path).await {
        Ok(bytes) => bytes,
        Err(err) => {
            tracing::warn!("Failed to read bytes from quarantine image {:?}", err);
            return Err(Box::new(err));
        }
    };

    Ok(bytes)
}

/**
 * Every 30 minutes deletes temporary images that are over 30 minutes old.
 */
pub async fn init_temporary_image_upload_cleanup() {
    let mut interval = interval(Duration::from_secs(1800));

    loop {
        interval.tick().await;

        tracing::info!("Cleaning up temporary image uploads.");
        let mut storage_path = get_temporary_storage_path().await;
        if let Ok(mut entries) = tokio::fs::read_dir(storage_path).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Ok(metadata) = entry.metadata().await {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(elapsed) = modified.elapsed() {
                            if elapsed.as_secs() > 1800 {
                                if let Err(error) = tokio::fs::remove_file(entry.path()).await {
                                    tracing::warn!("Error occurred when removing temporary image upload. {:?}", error);
                                }
                            }
                        }
                    }
                }
            }
        }

        tracing::info!("Cleaning up quarantine image uploads.");
        storage_path = get_quarantine_storage_path().await;
        if let Ok(mut entries) = tokio::fs::read_dir(storage_path).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Ok(metadata) = entry.metadata().await {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(elapsed) = modified.elapsed() {
                            if elapsed.as_secs() > 3600 {
                                if let Err(error) = tokio::fs::remove_file(entry.path()).await {
                                    tracing::warn!("Error occurred when removing quarantine image upload. {:?}", error);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

}

pub fn get_file_extension(filename: &str) -> String {
    filename.split(".").collect::<Vec<_>>().last().unwrap().to_string()
}

/**
 * Move the temporary image to the quarantine folder for scanning.
 */
pub async fn transfer_temporary_image_to_quarantine(
    temporary_image_filename: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let temporary_storage_path = get_temporary_storage_path().await;
    let temporary_image_path = temporary_storage_path.join(temporary_image_filename);

    if !temporary_image_path.exists() {
        return Err(
            Box::new(
                io::Error::new(io::ErrorKind::Other, "Uploaded image not found.")
            )
        );
    }

    let quarantine_image_path = get_quarantine_storage_path().await.join(
        &temporary_image_filename
    );

    if let Err(error) = tokio::fs::rename(&temporary_image_path, &quarantine_image_path).await {
        tracing::warn!("Error occurred when transferring temporary image to quarantine path {:?}", error);
        return Err(
            Box::new(
                io::Error::new(io::ErrorKind::Other, "Move from temporary to quarantine folder failed.")
            )
        );
    }

    Ok(())
}

/**
 * Upload image from the temporary upload storage to the IPFS node.
 */
pub async fn transfer_image_to_ipfs(
    bytes: Vec<u8>,
    filename: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {

    let secrets = secrets_config();
    let base_url = format!(r#"{}//{}:{}"#, secrets.ipfs.protocol, secrets.ipfs.host, secrets.ipfs.port);

    let file_part = Part::bytes(bytes).file_name(filename.to_string());
    let form = Form::new()
        .part("file", file_part)
        .text("wrap-with-directory", "true")
        .text("pin", "true")
        .text("progress", "false");

    let upload_file_response = match reqwest::Client::new()
        .post(format!("{base_url}/api/v0/add"))
        .multipart(form)
        .send()
        .await {
        Ok(response) => response,
        Err(err) => {
            tracing::warn!("Failed to send request to upload temporary image to IPFS {:?}", err);
            return Err(Box::new(err));
        }
    };
    let error_for_status_response = match upload_file_response.error_for_status() {
        Ok(response) => response,
        Err(err) => {
            tracing::warn!("IPFS node returned an error status when uploading an image {:?}", err);
            return Err(Box::new(err));
        }
    };

    let body_text_parse: String = match error_for_status_response.text().await {
        Ok(response) => response,
        Err(err) => {
            tracing::warn!("Error parsing IPFS node API response {:?}", err);
            return Err(Box::new(err));
        }
    };

    let mut body_parse_response: Option<IpfsAddResponse> = None;
    for line in body_text_parse.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(parsed_value) = serde_json::from_str::<IpfsAddResponse>(line) {
            if let Some(name) = &parsed_value.name {
                if name == filename {
                    body_parse_response = Some(parsed_value);
                    break;
                }
            }
        }
    }

    let cid = match body_parse_response {
        Some(body_parse_response) => {
            match body_parse_response.hash {
                Some(cid) => cid,
                None => {
                    tracing::warn!("Missing CID from IFPS add response.");
                    return Err(Box::new(io::Error::new(io::ErrorKind::Other, "Missing CID.")));
                }
            }
        },
        None => {
            tracing::warn!("Error parsing IPFS node API response.");
            return Err(Box::new(io::Error::new(io::ErrorKind::Other, "Missing CID.")));
        }
    };

    Ok(cid)
}

pub async fn add_ipfs_file_to_gifs_folder(
    cid: &str,
    filename: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {

    let secrets = secrets_config();
    let base_url = format!(r#"{}//{}:{}"#, secrets.ipfs.protocol, secrets.ipfs.host, secrets.ipfs.port);

    let add_file_to_folder_response = match reqwest::Client::new()
        .post(format!("{base_url}/api/v0/files/cp?arg=/ipfs/{cid}&arg=/opengifs/{filename}"))
        .send()
        .await {
        Ok(response) => response,
        Err(err) => {
            tracing::warn!("Failed to move image into folder in IPFS {:?}", err);
            return Err(Box::new(err));
        }
    };
    if let Err(err) = add_file_to_folder_response.error_for_status() {
        tracing::warn!("IPFS node returned an error status when moving image to folder {:?}", err);
        
        return Err(Box::new(err));
    }

    Ok(())
}

