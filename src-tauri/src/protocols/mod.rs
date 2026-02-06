pub mod common;
pub mod thumb;
pub mod image;
pub mod audio;
pub mod video;
pub mod font;
pub mod model;
pub mod placeholders;



/// Registration helper to keep lib.rs clean
pub fn register_all<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder
        .register_uri_scheme_protocol("thumb", move |ctx, request| {
            thumb::handler(ctx.app_handle(), &request)
        })
        .register_uri_scheme_protocol("image", move |_ctx, request| {
            image::handler(&request)
        })
        .register_uri_scheme_protocol("audio", move |_ctx, request| {
            audio::handler(&request)
        })
        .register_uri_scheme_protocol("video", move |_ctx, request| {
            video::handler(&request)
        })
        .register_uri_scheme_protocol("font", move |_ctx, request| {
            font::handler(&request)
        })
        .register_uri_scheme_protocol("model", move |_ctx, request| {
            model::handler(&request)
        })
        .register_uri_scheme_protocol("document", move |_ctx, request| {
            placeholders::document_handler(&request)
        })
        .register_uri_scheme_protocol("ebook", move |_ctx, request| {
            placeholders::ebook_handler(&request)
        })
        .register_uri_scheme_protocol("code", move |_ctx, request| {
            placeholders::code_handler(&request)
        })
}
