#![feature(async_closure)]

use bytes::BufMut;
use futures::TryStreamExt;
use humansize::{file_size_opts, FileSize};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::path::PathBuf;
use uuid::Uuid;
use warp::{
    http::{HeaderValue, StatusCode},
    multipart::{FormData, Part},
    Filter, Rejection, Reply,
};

#[derive(Debug, Serialize, Deserialize)]
struct FileMetadata {
    original_name: String,
    mime_type: String,
}

fn uploads_dir() -> PathBuf {
    PathBuf::from(&std::env::var_os("PICO_UPLOADS").expect("Set PICO_UPLOADS env var to proceed"))
}

async fn free_space() -> Result<impl Reply, Rejection> {
    let df = fs2::free_space(uploads_dir()).unwrap();
    Ok(df.file_size(file_size_opts::BINARY).unwrap())
}

async fn upload(form: FormData) -> Result<impl Reply, Rejection> {
    let parts: Vec<Part> = form.try_collect().await.map_err(|e| {
        eprintln!("form error: {}", e);
        warp::reject::reject()
    })?;

    for p in parts {
        if p.name() == "file" {
            let uuid_str = Uuid::new_v4().to_string();
            let original_name = p.filename().unwrap_or(&uuid_str).to_string();
            let mime_type = p
                .content_type()
                .unwrap_or("application/octet-stream")
                .to_string();

            let file_contents = p
                .stream()
                .try_fold(Vec::new(), |mut vec, data| {
                    vec.put(data);
                    async move { Ok(vec) }
                })
                .await
                .map_err(|e| {
                    eprintln!("reading file error: {}", e);
                    warp::reject::reject()
                })?;

            let mut d = uploads_dir();
            d.push(uuid_str.clone());
            tokio::fs::write(&d, file_contents).await.map_err(|e| {
                eprint!("error writing file: {}", e);
                warp::reject::reject()
            })?;

            d.set_extension("meta.json");
            tokio::fs::write(
                &d,
                serde_json::to_string(&FileMetadata {
                    original_name,
                    mime_type,
                })
                .unwrap(),
            )
            .await
            .map_err(|e| {
                eprint!("error writing file: {}", e);
                warp::reject::reject()
            })?;

            return Ok(uuid_str);
        }
    }

    return Err(warp::reject::reject());
}

fn sanitize_name(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        if c.is_alphabetic() || c == '_' {
            result.push(c);
        } else if let Some(last) = result.chars().last() {
            if c == '.' && last != '.' {
                result.push(c);
            } else if last != '_' {
                result.push('_');
            }
        }
    }

    if result.is_empty() {
        "unnamed".to_string()
    } else {
        result
    }
}

async fn set_download_headers(rfile: warp::fs::File) -> Result<impl Reply, Rejection> {
    let mut path = rfile.path().to_owned();
    path.set_extension("meta.json");
    let contents = tokio::fs::read(path).await.map_err(|e| {
        eprint!("error reading metadata: {}", e);
        warp::reject::reject()
    })?;
    let metadata: FileMetadata = serde_json::from_slice(&contents).expect("Corrupted FileMetadata");

    Ok(warp::reply::with_header(
        warp::reply::with_header(
            rfile,
            "Content-Disposition",
            HeaderValue::from_str(&format!(
                r#"attachment; filename="{}""#,
                sanitize_name(&metadata.original_name)
            ))
            .unwrap(),
        ),
        "Content-Type",
        HeaderValue::from_str(&metadata.mime_type).unwrap(),
    ))
}

async fn handle_rejection(err: Rejection) -> std::result::Result<impl Reply, Infallible> {
    let (code, message) = if err.is_not_found() {
        (StatusCode::NOT_FOUND, "Not Found".to_string())
    } else if err.find::<warp::reject::PayloadTooLarge>().is_some() {
        (StatusCode::BAD_REQUEST, "Payload too large".to_string())
    } else {
        eprintln!("unhandled error: {:?}", err);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal Server Error".to_string(),
        )
    };

    Ok(warp::reply::with_status(message, code))
}

#[tokio::main]
async fn main() {
    let port: u16 = std::env::args()
        .skip(1)
        .last()
        .map(|p| p.parse().expect("Port must be an integer"))
        .unwrap_or(8000);

    let dir = uploads_dir();

    let upload_route = warp::path("upload")
        .and(warp::post())
        .and(warp::multipart::form().max_length(5_000_000))
        .and_then(upload);

    let download_route = warp::path("file")
        .and(warp::fs::dir(dir))
        .and_then(async move |r| set_download_headers(r).await);

    let free_space_route = warp::path("free_space").and_then(free_space);

    let index_route = warp::path::end().and(warp::fs::file("static/index.html"));

    let static_route = warp::path("static").and(warp::fs::dir("static"));

    let router = upload_route
        .or(download_route)
        .or(free_space_route)
        .or(index_route)
        .or(static_route)
        .recover(handle_rejection);

    warp::serve(router).run(([127, 0, 0, 1], port)).await;
}
