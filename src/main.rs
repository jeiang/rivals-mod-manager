#![deny(clippy::all)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod app;

#[tokio::main]
async fn main() -> iced::Result {
    iced::run(update, view)
}
