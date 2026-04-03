#![recursion_limit = "256"]
mod app;
mod pages;

fn main() {
    dioxus::launch(app::App);
}
