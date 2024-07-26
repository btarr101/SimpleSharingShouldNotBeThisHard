use maud::{html, Markup};

use super::page::page;

pub fn not_found() -> Markup {
    page(html! {
        h1 { "Not Found" }
    })
}
