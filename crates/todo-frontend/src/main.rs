mod api;
mod app;
mod components;
mod speech;

use app::App;
use leptos::*;

fn main() {
    console_log::init_with_level(log::Level::Debug).expect("Failed to init logger");
    mount_to_body(|| view! { <App /> });
}
