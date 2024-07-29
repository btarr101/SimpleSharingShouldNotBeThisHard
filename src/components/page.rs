use chrono::Utc;
use maud::{html, Markup, DOCTYPE};

pub fn page(content: Markup) -> Markup {
    let css_source = if cfg!(debug_assertions) {
        format!("/public/style.css?version={}", Utc::now())
    } else {
        // Technially this doesn't need to be a string,
        // and I could organize the conditional compilation
        // so it's a static &str, but meh.
        "/public/style.css".into()
    };

    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf8";
                meta name="description" content="Upload files here for quick an easy temporary sharing!";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "SimpleSharingShouldNotBeThisHard.com" }
                link rel="stylesheet" type="text/css" href=(css_source);
                script src="/public/js/htmx.min.js" {};
                script src="/public/js/hyperscript.min.js" {};
            }
            body hx-boost="true" {
                header {
                    center {
                        h1 { "SimpleSharingShouldNotBeThisHard.com" }
                    }
                }
                main {
                    (content)
                }
                footer {
                    center {
                        h2 { "Why should you use this site?" }
                    }
                    article {
                        section {
                            center {
                                img src="/public/incognito-svgrepo-com.svg" alt="incognito" width="64px" height="64px";
                                h3 { "No account needed" }
                                p { "You upload files completely anonymously." }
                            }
                        }
                        section {
                            center {
                                img src="/public/infinity-svgrepo-com.svg" alt="infinity" width="64px" height="64px";
                                h3 { "No size limits" }
                                p { "The whole point of making this was due to my annoyance with size restrictions in messaging apps, so yeah go crazy." }
                            }
                        }
                        section {
                            center {
                                img src="/public/upload-svgrepo-com.svg" alt="upload" width="64px" height="64px";
                                h3 { "Just sharing" }
                                p { "This site prioritizes a simple streamlined experience over all else, too often applications are filled with bloat that adds unneeded mental strain." }
                            }
                        }
                        section {
                            center {
                                img src="/public/github-svgrepo-com.svg" alt="github" width="64px" height="64px";
                                h3 { "Open source" }
                                p { "Any specific questions? Read the code yourself!"
                                    @if let Some(github_site) = option_env!("GITHUB_SITE") {
                                        " ("
                                        a href=(github_site) { "link" }
                                        ")"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
