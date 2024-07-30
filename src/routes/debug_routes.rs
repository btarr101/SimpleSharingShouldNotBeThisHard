use axum::http::StatusCode;
use maud::Markup;

use crate::components::error_page::error_page;

pub async fn get_500() -> (StatusCode, Markup) {
    error_page(
        StatusCode::INTERNAL_SERVER_ERROR,
        "This is a debug 500 page.",
    )
}
