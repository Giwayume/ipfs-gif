use std::sync::atomic::{ AtomicBool, Ordering };
use std::{ error::Error, io::{ Cursor, Read } };

use nsfw::{ create_model, examine, model::{ Classification, Metric } };

use crate::database::{ self, Gif, QuarantineScanResult };
use crate::util::format;
use crate::util::image_upload::{
    get_quarantine_image,
    get_file_extension,
    transfer_image_to_ipfs,
    add_ipfs_file_to_gifs_folder,
};

static RUNNING: AtomicBool = AtomicBool::new(false);

pub fn start_scanning_quarantine() {
    if RUNNING
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }

    tokio::spawn(async move {
        tracing::info!("Image quarantine scanner started.");
        let ok = runner().await;
        if ok {
            tracing::info!("Image quarantine scanner finished.");
        } else {
            tracing::info!("Image quarantine scanner stopped with an error."); 
        }
        RUNNING.store(false, Ordering::Release);
    });
}

async fn runner() -> bool {
    let bytes = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/models/gantman-nsfw.onnx"));
    let model = match create_model(Cursor::new(bytes)) {
        Ok(model) => model,
        Err(_) => {
            tracing::warn!("Failed to load the gantman-nsfw.onnx model.");
            return false;
        },
    };

    let _ = database::delete_old_quarantine_gifs().await;

    loop {
        let gif = match database::get_next_quarantined_gif().await {
            Ok(gif) => gif,
            Err(_) => {
                return true;
            },
        };

        let quarantine_filename = match gif.quarantine_id
            .replace("qt-", "")
            .rsplit_once("-") {
            Some((left, right)) => format!("{left}.{right}"),
            _ => String::from(""),
        };

        let bytes = match get_quarantine_image(&quarantine_filename).await {
            Ok(bytes) => bytes,
            Err(_) => {
                if let Err(_) = database::update_gif_quarantine_scan_result(gif.id, QuarantineScanResult::MissingImage).await {
                    tracing::warn!("Failed to retrieve a quarantined image, and can't mark the database entry with id {}", gif.id);
                    return false;
                } else {
                    continue;
                }
            },
        };

        let image = match image::load_from_memory(&bytes) {
            Ok(image) => image,
            Err(_) => {
                if let Err(_) = database::update_gif_quarantine_scan_result(gif.id, QuarantineScanResult::ImageParseFailed).await {
                    tracing::warn!("Failed to decode quarantined image, and can't mark the database entry with id {}", gif.id);
                    return false;
                } else {
                    continue;
                }
            }
        };
        let image_buffer = image.to_rgba8();

        let predictions = match examine(&model, &image_buffer) {
            Ok(predictions) => Some(predictions),
            Err(_) => None,
        };
        if predictions.is_none() {
            if let Err(_) = database::update_gif_quarantine_scan_result(gif.id, QuarantineScanResult::ScanFailed).await {
                tracing::warn!("Failed to create predictions for quarantined image, and can't mark the database entry with id {}", gif.id);
                return false;
            } else {
                continue;
            }
        }

        if is_prediction_above_nsfw_threshold(predictions.unwrap()) {
            if let Err(_) = database::update_gif_quarantine_scan_result(gif.id, QuarantineScanResult::ScanFailed).await {
                tracing::warn!("Quarantine scan failed for image image, and can't mark the database entry with id {}", gif.id);
                return false;
            } else {
                continue;
            }
        }

        let filename = create_filename(
            &gif.description,
            &quarantine_filename,
        );

        let ipfs_transfer_result = transfer_image_to_ipfs(
            bytes,
            &filename
        ).await;
        if let Err(_transfer_error) = ipfs_transfer_result {
            if let Err(_) = database::update_gif_quarantine_scan_result(gif.id, QuarantineScanResult::IpfsTransferFailed).await {
                tracing::warn!("Quarantine scan failed for image image, and can't mark the database entry with id {}", gif.id);
                return false;
            } else {
                continue;
            }
        }
        let cid = ipfs_transfer_result.unwrap();

        let existing_gif = match database::get_gif_by_cid(&cid).await {
            Ok(gif) => Some(gif),
            Err(_) => None,
        };
        if existing_gif.is_some() {
            if let Err(_) = database::update_gif_quarantine_scan_result(gif.id, QuarantineScanResult::IpfsDuplicate).await {
                tracing::warn!("The user uploaded a duplicate image, and can't mark the database entry with id {}", gif.id);
                return false;
            } else {
                continue;
            }
        }

        let _ = add_ipfs_file_to_gifs_folder(&cid, &filename).await;

        let _ = database::update_gif_cid(gif.id, &cid).await;
    }
}

fn is_prediction_above_nsfw_threshold(predictions: Vec<Classification>) -> bool {
    for prediction in predictions {
        match prediction.metric {
            Metric::Hentai => {
                if prediction.score > 0.9 {
                    return true;
                }
            },
            Metric::Porn => {
                if prediction.score > 0.9 {
                    return true;
                }
            },
            Metric::Sexy => {
                if prediction.score > 0.9 {
                    return true;
                }
            },
            _ => (),
        }
    }
    return false;
}

fn create_filename(description: &str, temporary_filename: &str) -> String {
    format!(
        "{}.{}",
        format::to_kebab_case(format::truncate(description, 250)),
        get_file_extension(temporary_filename),
    )
}
