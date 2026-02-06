use super::common::{decode_path, extract_path_part, serve_file};
use tauri::http::{header, Response, Request};
use std::path::PathBuf;

pub fn document_handler(request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let uri = request.uri().to_string();
    let path_part = extract_path_part(&uri, "document");
    handle_generic(path_part, request)
}

pub fn ebook_handler(request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let uri = request.uri().to_string();
    let path_part = extract_path_part(&uri, "ebook");
    handle_generic(path_part, request)
}

pub fn code_handler(request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let uri = request.uri().to_string();
    let path_part = extract_path_part(&uri, "code");
    handle_generic(path_part, request)
}

fn handle_generic(path_part: String, request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let decoded_path = decode_path(&path_part);
    let mut full_path = PathBuf::from(&decoded_path);

    if !full_path.is_absolute() && cfg!(unix) {
        if !path_part.starts_with('/') {
            full_path = PathBuf::from("/").join(full_path);
        }
    }

    let range = request.headers().get(header::RANGE);
    match serve_file(&full_path, range) {
        Ok(res) => res,
        Err(res) => res,
    }
}
