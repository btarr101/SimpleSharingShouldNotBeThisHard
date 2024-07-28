use maud::{html, Markup, DOCTYPE};

pub fn page(content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf8";
                meta name="description" content="Upload files here for quick an easy temporary sharing!";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "SimpleSharingShouldNotBeThisHard.com" }
            }
            body {
                header {
                    h1 { "SimpleSharingShouldNotBeThisHard.com" }
                }
                (content)
                footer {
                    article {
                        h2 { "Why should you use this site?" }
                        section {
                            img src="/public/incognito-svgrepo-com.svg" alt="incognito" width="64px" height="64px";
                            h3 { "No account needed" }
                            p { "You upload files completely anonymously." }
                        }
                        section {
                            img src="/public/infinity-svgrepo-com.svg" alt="infinity" width="64px" height="64px";
                            h3 { "No size limits" }
                            p { "The whole point of making this was due to my annoyance with size restrictions in messaging apps, so yeah go crazy." }
                        }
                        section {
                            img src="/public/upload-svgrepo-com.svg" alt="upload" width="64px" height="64px";
                            h3 { "Just sharing" }
                            p { "This site prioritizes a simple streamlined experience over all else, too often applications are filled with bloat that adds unneeded mental strain." }
                        }
                        section {
                            img src="/public/github-svgrepo-com.svg" alt="github" width="64px" height="64px";
                            h3 { "Open source" }
                            p { "Any specific questions? Read the code yourself! (LINK HERE)" }
                        }
                    }
                }
            }
        }
    }
}
