use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Device, Host};
use log::{debug, info, warn};
use std::collections::HashMap;
use std::thread;
use std::time::{Duration, Instant};

use crate::config::{Config, DeviceType};

pub struct AudioDevices {
    devices: HashMap<String, Device>,
}

impl AudioDevices {
    pub fn get(&self, name: &str) -> Result<&Device> {
        self.devices
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Device '{}' not found", name))
    }

    pub fn find_all(config: &Config, host: &Host) -> Result<Self> {
        if config.device_wait.enabled {
            Self::find_with_retry(config, host)
        } else {
            Self::find_immediate(config, host)
        }
    }

    fn find_immediate(config: &Config, host: &Host) -> Result<Self> {
        info!("Searching for audio devices...");

        let mut devices = HashMap::new();

        for (alias, device_config) in &config.devices {
            let device = Self::find_device(host, &device_config.name)
                .with_context(|| format!("Device '{}' not found: {}", alias, device_config.name))?;

            Self::verify_device_type(&device, &device_config.device_type, alias)?;

            info!("Found {} device: {}", alias, device_config.name);
            devices.insert(alias.clone(), device);
        }

        Ok(Self { devices })
    }

    fn find_with_retry(config: &Config, host: &Host) -> Result<Self> {
        let wait_config = &config.device_wait;
        let start_time = Instant::now();
        let max_duration = Duration::from_secs(wait_config.max_wait_time);
        let retry_interval = Duration::from_secs(wait_config.retry_interval);

        info!(
            "Waiting for audio devices (max {}s)...",
            wait_config.max_wait_time
        );

        let mut devices = HashMap::new();
        let mut missing: Vec<String> = config.devices.keys().cloned().collect();

        while start_time.elapsed() < max_duration && !missing.is_empty() {
            let mut found_this_round = Vec::new();

            for alias in &missing {
                if let Some(device_config) = config.devices.get(alias) {
                    if let Some(device) = Self::find_device(host, &device_config.name) {
                        if Self::verify_device_type(&device, &device_config.device_type, alias)
                            .is_ok()
                        {
                            info!("Found {} device: {}", alias, device_config.name);
                            devices.insert(alias.clone(), device);
                            found_this_round.push(alias.clone());
                        }
                    }
                }
            }

            for alias in found_this_round {
                missing.retain(|x| x != &alias);
            }

            if missing.is_empty() {
                info!("All devices found");
                return Ok(Self { devices });
            }

            let elapsed = start_time.elapsed().as_secs();
            debug!(
                "Waiting for devices... ({}s elapsed, {} missing)",
                elapsed,
                missing.len()
            );

            thread::sleep(retry_interval);
        }

        if !missing.is_empty() {
            if wait_config.allow_partial {
                warn!("Some devices not found after timeout: {:?}", missing);
                warn!("Continuing with partial device set (allow_partial=true)");

                if devices.is_empty() {
                    return Err(anyhow::anyhow!("No devices found, cannot continue"));
                }
            } else {
                return Err(anyhow::anyhow!(
                    "Devices not found after {}s timeout: {:?}",
                    wait_config.max_wait_time,
                    missing
                ));
            }
        }

        Ok(Self { devices })
    }

    fn verify_device_type(device: &Device, expected_type: &DeviceType, alias: &str) -> Result<()> {
        match expected_type {
            DeviceType::Input => {
                device
                    .default_input_config()
                    .map_err(|_| anyhow::anyhow!("Device '{}' is not an input device", alias))?;
            }
            DeviceType::Output => {
                device
                    .default_output_config()
                    .map_err(|_| anyhow::anyhow!("Device '{}' is not an output device", alias))?;
            }
        }
        Ok(())
    }

    fn find_device(host: &Host, name_pattern: &str) -> Option<Device> {
        host.devices()
            .ok()?
            .find(|d| d.name().unwrap_or_default().contains(name_pattern))
    }

    pub fn list_available(host: &Host) -> Vec<String> {
        let mut devices = Vec::new();

        if let Ok(available) = host.devices() {
            for device in available {
                if let Ok(name) = device.name() {
                    devices.push(name);
                }
            }
        }

        devices
    }
}
