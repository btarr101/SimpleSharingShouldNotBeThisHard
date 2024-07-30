use axum::http::StatusCode;
use maud::{html, Markup};

use super::page::page;

pub fn error_page(status_code: StatusCode, error_text: &str) -> (StatusCode, Markup) {
    (
        status_code,
        page(
            html! {
                center {
                    h1 { (status_code) }
                    p { (error_text) }
                }
            },
            false,
        ),
    )
}
