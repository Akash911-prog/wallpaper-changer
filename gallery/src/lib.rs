// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod config;

use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
};

use crate::config::Config;

slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    let config = load_config()?;

    println!("config: {:#?}", config);

    let ui = AppWindow::new()?;

    let paths = scan_folder(config);

    ui.run()?;

    Ok(())
}

fn scan_folder(config: Config) -> Result<Vec<PathBuf>, std::io::Error> {
    let paths: Vec<PathBuf> = fs::read_dir(config.wallpaper_dir)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();

    Ok(paths)
}

pub fn init_ui() {
    let _ = main();
}

pub fn load_config() -> Result<Config, config::ParseError> {
    let config_path = Path::new(r"F:\prod-poject\wallpaper-changer\gallery\config.txt");
    let config_str = fs::read_to_string(config_path).unwrap();
    let config = Config::from_env_str(&config_str);

    config
}
