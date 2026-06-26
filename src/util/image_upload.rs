use std::{ io::{ self, Cursor }, env, error::Error, path::PathBuf };
use axum::body::Bytes;
use image::{ ImageReader, DynamicImage, ImageFormat };
use tokio::time::{ interval, Duration };
use uuid::Uuid;

pub static TEMPORARY_IMAGE_DIRECTORY: &str = "uploads/assets/images/tmp";
pub static IMAGE_UPLOAD_BASE_DIRECTORY: &str = "uploads/assets/images";

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
 * Returns the base path to the images upload folder where images are moved to
 * after a successful form submission that includes an image upload.
 */
async fn get_permanent_storage_base_path() -> PathBuf {
    let path = if cfg!(debug_assertions) {
        env::current_dir().unwrap().join(IMAGE_UPLOAD_BASE_DIRECTORY)
    } else {
        env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .join(IMAGE_UPLOAD_BASE_DIRECTORY)
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
    let file_extension = match content_type.as_str() {
        "image/apng" => Some("apng"),
        "image/avif" => Some("avif"),
        "image/bmp" => Some("bmp"),
        "image/gif" => Some("gif"),
        "image/jpg" => Some("jpeg"),
        "image/jpeg" => Some("jpeg"),
        "image/png" => Some("png"),
        "image/svg+xml" => Some("svg"),
        "image/tiff" => Some("tiff"),
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
    let _ = reader.format().ok_or("Unknown image format")?;
    let _ = reader.decode()?;

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
 * Every 30 minutes deletes temporary images that are over 30 minutes old.
 */
pub async fn init_temporary_image_upload_cleanup() {
    let mut interval = interval(Duration::from_secs(1800));

    loop {
        interval.tick().await;

        tracing::info!("Cleaning up temporary image uploads.");
        let storage_path = get_temporary_storage_path().await;
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
    }

}

fn get_file_extension(filename: &str) -> String {
    filename.split(".").collect::<Vec<_>>().last().unwrap().to_string()
}

/**
 * Moves an image from the temporary upload storage to a permanent folder.
 */
pub async fn transfer_temporary_image_upload(
    temporary_image_filename: &str,
    permanent_path: &str,
    permanent_image_filename: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let temporary_storage_path = get_temporary_storage_path().await;
    let temporary_image_path = temporary_storage_path.join(temporary_image_filename);

    if !temporary_image_path.exists() {
        return Err(
            Box::new(
                io::Error::new(io::ErrorKind::Other, "Uploaded image not found.")
            )
        );
    }

    let file_extension = get_file_extension(temporary_image_filename);
    let permanent_filename_with_extension = format!("{}.{}", &permanent_image_filename, &file_extension);

    // Generate an image thumbnail.
    let data = tokio::fs::read(&temporary_image_path).await?;
    let reader = ImageReader::new(Cursor::new(&data)).with_guessed_format()?;
    let format: ImageFormat = reader.format().ok_or("Unknown image format")?;
    let image: DynamicImage = reader.decode()?;

    let target_area: f32 = 152.0 * 152.0;
    let original_width = image.width() as f32;
    let original_height = image.height() as f32;
    let scale_factor = (target_area / (original_width / original_height)).sqrt();
    let thumbnail_width = (original_width / original_height * scale_factor).round() as u32;
    let thumbnail_height = scale_factor.round() as u32;

    let thumbnail = image.resize_exact(thumbnail_width, thumbnail_height, image::imageops::FilterType::Lanczos3);
    let mut thumbnail_data = Vec::new();
    thumbnail.write_to(&mut Cursor::new(&mut thumbnail_data), format)?;

    let thumbnail_folder_path = get_permanent_storage_base_path()
        .await
        .join(permanent_path)
        .join("thumbs");
    if let Ok(_) = tokio::fs::metadata(thumbnail_folder_path).await {
        let thumbnail_file_path = get_permanent_storage_base_path()
            .await
            .join(permanent_path)
            .join("thumbs")
            .join(&permanent_filename_with_extension);
        if let Err(error) = tokio::fs::write(thumbnail_file_path, thumbnail_data).await {
            tracing::warn!("Error occurred when writing thumbnail to permanent path {:?}", error);
            return Err(
                Box::new(
                    io::Error::new(io::ErrorKind::Other, "Write of thumbnail failed.")
                )
            );
        };
    }

    // Move the temporary image to permanent location.
    let final_path = get_permanent_storage_base_path().await.join(permanent_path).join(
        &permanent_filename_with_extension
    );
    if let Err(error) = tokio::fs::rename(&temporary_image_path, &final_path).await {
        tracing::warn!("Error occurred when transferring temporary image to permanent path {:?}", error);
        return Err(
            Box::new(
                io::Error::new(io::ErrorKind::Other, "Move from temporary to permanent folder failed.")
            )
        );
    }

    Ok(permanent_filename_with_extension)
}
