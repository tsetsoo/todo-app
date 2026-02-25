// Leptos view! macro requires wildcard import of leptos::*
#![allow(clippy::wildcard_imports)]

mod api;
mod app;
mod components;
mod speech;
mod ws;

use app::App;
use leptos::*;

fn main() {
    console_log::init_with_level(log::Level::Debug).expect("Failed to init logger");
    mount_to_body(|| view! { <App /> });
}
