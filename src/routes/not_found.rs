use axum::http::StatusCode;
use maud::Markup;

use crate::components::error_page::error_page;

pub async fn not_found() -> (StatusCode, Markup) {
    error_page(
        StatusCode::NOT_FOUND,
        "The page you were looking for does not exist.",
    )
}
