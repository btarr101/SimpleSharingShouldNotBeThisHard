use maud::{html, Markup, DOCTYPE};

pub fn page(content: Markup) -> Markup
{
    html! {
        head {
            (DOCTYPE)
            meta charset="utf8";
            title { "tempShare" }
        }
        body {
            header {
                h1 { "tempShare" }
            }
            (content)
        }
    }
}
