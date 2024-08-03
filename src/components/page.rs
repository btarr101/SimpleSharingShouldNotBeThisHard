use chrono::Utc;
use maud::{html, Markup, PreEscaped, DOCTYPE};

pub fn page(content: Markup, is_index: bool) -> Markup {
    let css_source = if cfg!(debug_assertions) {
        // Cache buster for local development
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
                meta name="msapplication-TileColor" content="#da532c";
                meta name="theme-color" content="#ffffff";

                title { "SimpleSharingShouldNotBeThisHard.com" }

                link rel="stylesheet" type="text/css" href=(css_source);
                link rel="apple-touch-icon" sizes="180x180" href="/public/apple-touch-icon.png";
                link rel="icon" type="image/png" sizes="32x32" href="/public/favicon-32x32.png";
                link rel="icon" type="image/png" sizes="16x16" href="/public/favicon-16x16.png";
                link rel="manifest" href="/public/site.webmanifest";
                link rel="mask-icon" href="/public/safari-pinned-tab.svg" color="#5bbad5";

                script src="/public/js/htmx.min.js" defer {};
                script src="/public/js/hyperscript.min.js" defer {};
                script src="/public/js/htmx-ext-loading-states.js" defer {};
                script defer {
                    (PreEscaped(
r#"
function formatDuration(ms) {
    const time = {
        days: Math.floor(ms / 86400000),
        h: Math.floor(ms / 3600000) % 24,
        m: Math.floor(ms / 60000) % 60,
        s: Math.floor(ms / 1000) % 60,
    };
    return Object.entries(time)
    .filter(val => val[1] !== 0)
    .map(([key, val]) => `${val}${key}`)
    .join(' ');
};
"#
                    ))
                }
            }
            body hx-boost="true" hx-ext="loading-states" {
                header {
                    center {
                        h1 {
                            "Simple" wbr;
                            "Sharing" wbr;
                            "Should" wbr;
                            "Not" wbr;
                            "Be" wbr;
                            "This" wbr;
                            "Hard" wbr;
                            ".shuttleapp" wbr;
                            ".rs"
                        }
                        @if !is_index {
                            nav {
                                a href="/" disabled { "Share a new file" }
                            }
                            br;
                        }
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
