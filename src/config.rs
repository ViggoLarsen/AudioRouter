use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub devices: HashMap<String, DeviceConfig>,
    pub routing: HashMap<String, RouteConfig>,
    pub audio: AudioConfig,
    pub logging: LoggingConfig,
    pub device_wait: DeviceWaitConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DeviceConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub device_type: DeviceType,
    pub buffer_size: u32,
    pub primary_buffer: usize,
    pub gain: f32,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    Input,
    Output,
}

impl fmt::Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceType::Input => write!(f, "input"),
            DeviceType::Output => write!(f, "output"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RouteConfig {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AudioConfig {
    pub prefill_samples: usize,
    pub keep_alive_sleep_ms: u64,
    pub stereo_to_mono_mix_ratio: f32,
    pub audio_sample_min: f32,
    pub audio_sample_max: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LoggingConfig {
    pub level: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeviceWaitConfig {
    pub enabled: bool,
    pub max_wait_time: u64,
    pub retry_interval: u64,
    pub allow_partial: bool,
}

impl Config {
    pub fn load() -> Result<Self> {
        let exe_path = env::current_exe().context("Failed to get executable path")?;

        let config_path = exe_path
            .parent()
            .context("Failed to get executable directory")?
            .join("config.yaml");

        if !config_path.exists() {
            return Err(anyhow::anyhow!(
                "Config file not found at: {}. Please create a config.yaml file next to the executable.",
                config_path.display()
            ));
        }

        let config_str = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config from: {}", config_path.display()))?;

        let config: Config =
            serde_yaml::from_str(&config_str).context("Failed to parse config YAML")?;

        Ok(config)
    }

    pub fn get_config_dir() -> Result<PathBuf> {
        let exe_path = env::current_exe().context("Failed to get executable path")?;

        let dir = exe_path
            .parent()
            .context("Failed to get executable directory")?
            .to_path_buf();

        Ok(dir)
    }
}
