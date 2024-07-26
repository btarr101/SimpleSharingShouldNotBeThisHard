use maud::{html, Markup};

use super::page::page;

pub fn internal_server_error() -> Markup {
    page(html! {
        h1 { "Internal Server Error" }
    })
}
