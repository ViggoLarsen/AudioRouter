use anyhow::{Context, Result};
use log::info;
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

mod audio;
mod config;
mod devices;
mod logger;

#[cfg(windows)]
mod service;
#[cfg(windows)]
mod service_manager;

use config::Config;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            #[cfg(windows)]
            "install" => {
                return service_manager::install_service();
            }
            #[cfg(windows)]
            "uninstall" => {
                return service_manager::uninstall_service();
            }
            #[cfg(windows)]
            "service" => {
                return service::run_as_service();
            }
            "console" | "run" => {
                return run_console_mode();
            }
            "list-devices" => {
                return list_devices();
            }
            _ => {
                print_usage();
                return Ok(());
            }
        }
    }

    run_console_mode()
}

fn run_console_mode() -> Result<()> {
    let config = Config::load().context("Failed to load configuration")?;

    let log_path = Config::get_config_dir()?.join("logs.txt");
    logger::FileLogger::init(log_path.clone(), &config.logging.level)?;

    info!("Audio routing service started (console mode)");
    info!("Configuration loaded from config.yaml");
    info!("Logging to: {}", log_path.display());

    info!("Device configuration:");
    for (alias, device_config) in &config.devices {
        info!(
            "  {} ({}): {}",
            alias, device_config.device_type, device_config.name
        );
    }

    info!("Routing configuration:");
    for (route_name, route_config) in &config.routing {
        info!(
            "  {}: {} → {}",
            route_name, route_config.from, route_config.to
        );
    }

    let running = Arc::new(AtomicBool::new(true));
    let running_handle = running.clone();

    ctrlc::set_handler(move || {
        info!("Shutdown requested (Ctrl+C)");
        running_handle.store(false, Ordering::SeqCst);
    })?;

    info!("Press Ctrl+C to stop");

    audio::run_audio_routing(config, running)?;

    info!("Service stopped");
    Ok(())
}

fn list_devices() -> Result<()> {
    let host = cpal::default_host();

    println!("Available audio devices:");
    println!("========================");

    let devices = devices::AudioDevices::list_available(&host);

    if devices.is_empty() {
        println!("No audio devices found!");
    } else {
        for (i, device) in devices.iter().enumerate() {
            println!("{}. {}", i + 1, device);
        }
    }

    Ok(())
}

fn print_usage() {
    println!("Audio Router - Audio routing service");
    println!();
    println!("Usage:");
    println!("  audio_router                  Run in console mode");
    println!("  audio_router console          Run in console mode");
    println!("  audio_router list-devices     List available audio devices");

    #[cfg(windows)]
    {
        println!("  audio_router install          Install as Windows service");
        println!("  audio_router uninstall        Uninstall Windows service");
        println!("  audio_router service          Run as Windows service (internal use)");
    }
}
