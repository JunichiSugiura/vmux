use dioxus::prelude::*;

const CSS: Asset = asset!("/assets/style.css");
const ICON: Asset = asset!("/assets/icon.png");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Stylesheet { href: CSS }
        Hero {}
        Features {}
        Install {}
        Footer {}
    }
}

#[component]
fn Hero() -> Element {
    rsx! {
        section { class: "hero",
            img { src: ICON, alt: "Vmux icon", class: "hero-icon" }
            h1 { "Vmux" }
            p { class: "tagline", "Vibe Multiplexer — AI-native workspace combining browser and terminal panes." }
            div { class: "install-oneliner",
                code { "curl -fsSL https://vmux.ai/install | sh" }
                button {
                    class: "copy-btn",
                    onclick: move |_| {
                        spawn(async {
                            let _ = document::eval(r#"navigator.clipboard.writeText("curl -fsSL https://vmux.ai/install | sh")"#).await;
                        });
                    },
                    "Copy"
                }
            }
        }
    }
}

#[component]
fn Features() -> Element {
    let features = [
        (
            "Vibe Driven Development",
            "Talk to your workspace. Browse, run commands, edit files — all in one place.",
        ),
        (
            "Tmux-like Tiling",
            "Split, arrange, and manage browser and terminal panes in a single window.",
        ),
        (
            "Built-in Chromium",
            "Browse the web, read docs, and use web apps right next to your terminal.",
        ),
        (
            "3D Workspace",
            "Powered by Bevy game engine. Your workspace lives in a GPU-rendered 3D scene.",
        ),
    ];

    rsx! {
        section { class: "features",
            h2 { "Features" }
            div { class: "feature-grid",
                for (title, desc) in features {
                    div { class: "feature-card",
                        h3 { "{title}" }
                        p { "{desc}" }
                    }
                }
            }
        }
    }
}

#[component]
fn Install() -> Element {
    rsx! {
        section { class: "install",
            h2 { "Install" }

            div { class: "install-methods",
                div { class: "install-method",
                    h3 { "Quick Install" }
                    pre {
                        code { "curl -fsSL https://vmux.ai/install | sh" }
                    }
                }

                div { class: "install-method",
                    h3 { "Homebrew" }
                    pre {
                        code { "brew tap vmux-ai/vmux https://github.com/vmux-ai/vmux\nbrew install --cask vmux" }
                    }
                }

                div { class: "install-method",
                    h3 { "Download" }
                    p {
                        "Grab the latest "
                        code { ".dmg" }
                        " from "
                        a {
                            href: "https://github.com/vmux-ai/vmux/releases",
                            target: "_blank",
                            "GitHub Releases"
                        }
                        "."
                    }
                }
            }

            p { class: "requirement", "Requires macOS 13.0 (Ventura) or later." }
        }
    }
}

#[component]
fn Footer() -> Element {
    rsx! {
        footer {
            p {
                a {
                    href: "https://github.com/vmux-ai/vmux",
                    target: "_blank",
                    "GitHub"
                }
                " · "
                a {
                    href: "https://github.com/vmux-ai/vmux/blob/main/LICENSE",
                    target: "_blank",
                    "MIT License"
                }
            }
        }
    }
}
