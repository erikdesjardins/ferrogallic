use warp::{http, Reply};

pub fn bytes(data: &'static [u8], content_type: &'static str) -> impl Reply {
    http::Response::builder()
        .header(http::header::CONTENT_TYPE, content_type)
        .body(data)
}

pub fn string(data: &'static str, content_type: &'static str) -> impl Reply {
    bytes(data.as_bytes(), content_type)
}
