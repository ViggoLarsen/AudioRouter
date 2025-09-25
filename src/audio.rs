use anyhow::Result;
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{BufferSize, Stream, StreamConfig};
use log::{debug, error, info, warn};
use ringbuf::{HeapConsumer, HeapProducer, HeapRb};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::config::{Config, DeviceType};
use crate::devices::AudioDevices;

const NO_GAIN: f32 = 1.0;

struct AudioSettings {
    mix_ratio: f32,
    sample_min: f32,
    sample_max: f32,
}

struct AudioRoute {
    from_device: String,
    to_device: String,
    input_stream: Stream,
    output_stream: Stream,
}

pub fn run_audio_routing(config: Config, running: Arc<AtomicBool>) -> Result<()> {
    let host = cpal::default_host();
    let devices = AudioDevices::find_all(&config, &host)?;

    validate_routing(&config)?;

    let mut routes = Vec::new();

    for (buffer_index, (route_name, route_config)) in config.routing.iter().enumerate() {
        info!(
            "Setting up route: {} ({} -> {})",
            route_name, route_config.from, route_config.to
        );

        let from_device = devices.get(&route_config.from)?;
        let to_device = devices.get(&route_config.to)?;

        let from_device_config = config
            .devices
            .get(&route_config.from)
            .ok_or_else(|| anyhow::anyhow!("Device '{}' not found in config", route_config.from))?;
        let to_device_config = config
            .devices
            .get(&route_config.to)
            .ok_or_else(|| anyhow::anyhow!("Device '{}' not found in config", route_config.to))?;

        if from_device_config.device_type != DeviceType::Input {
            return Err(anyhow::anyhow!(
                "Route source '{}' must be an input device",
                route_config.from
            ));
        }
        if to_device_config.device_type != DeviceType::Output {
            return Err(anyhow::anyhow!(
                "Route destination '{}' must be an output device",
                route_config.to
            ));
        }

        let input_cfg = from_device.default_input_config()?;
        let output_cfg = to_device.default_output_config()?;

        info!(
            "  {} ({}): {} channels, {} Hz, format: {:?}",
            route_config.from,
            from_device_config.name,
            input_cfg.channels(),
            input_cfg.sample_rate().0,
            input_cfg.sample_format()
        );
        info!(
            "  {} ({}): {} channels, {} Hz, format: {:?}",
            route_config.to,
            to_device_config.name,
            output_cfg.channels(),
            output_cfg.sample_rate().0,
            output_cfg.sample_format()
        );

        if input_cfg.sample_rate() != output_cfg.sample_rate() {
            warn!(
                "Sample rate mismatch in route '{}': {} Hz -> {} Hz",
                route_name,
                input_cfg.sample_rate().0,
                output_cfg.sample_rate().0
            );
        }

        let buffer_size = from_device_config.primary_buffer;

        let rb = HeapRb::<f32>::new(buffer_size);
        let (mut producer, mut consumer): (HeapProducer<f32>, HeapConsumer<f32>) = rb.split();

        if buffer_index > 0 && config.audio.prefill_samples > 0 {
            debug!(
                "Pre-filling buffer for route '{}' with {} silence samples",
                route_name, config.audio.prefill_samples
            );
            for _ in 0..config.audio.prefill_samples {
                producer.push(0.0).ok();
            }
        }

        let buffer_size_config = BufferSize::Fixed(from_device_config.buffer_size);

        let gain = from_device_config.gain;

        if gain != NO_GAIN {
            info!("  Applying gain of {} to input", gain);
        }

        let in_channels = input_cfg.channels();
        let out_channels = output_cfg.channels();

        let from_name = route_config.from.clone();
        let to_name = route_config.to.clone();
        let audio_settings = AudioSettings {
            mix_ratio: config.audio.stereo_to_mono_mix_ratio,
            sample_min: config.audio.audio_sample_min,
            sample_max: config.audio.audio_sample_max,
        };

        let input_stream = from_device.build_input_stream(
            &StreamConfig {
                channels: input_cfg.channels(),
                sample_rate: input_cfg.sample_rate(),
                buffer_size: buffer_size_config,
            },
            move |data: &[f32], _| {
                handle_input_data(
                    data,
                    &mut producer,
                    in_channels,
                    out_channels,
                    gain,
                    &audio_settings,
                );
            },
            move |err| error!("Input error on '{}': {}", from_name, err),
            None,
        )?;

        let output_stream = to_device.build_output_stream(
            &StreamConfig {
                channels: output_cfg.channels(),
                sample_rate: output_cfg.sample_rate(),
                buffer_size: buffer_size_config,
            },
            move |data: &mut [f32], _| {
                for sample in data {
                    *sample = consumer.pop().unwrap_or(0.0);
                }
            },
            move |err| error!("Output error on '{}': {}", to_name, err),
            None,
        )?;

        routes.push(AudioRoute {
            from_device: route_config.from.clone(),
            to_device: route_config.to.clone(),
            input_stream,
            output_stream,
        });
    }

    for route in &routes {
        route.input_stream.play()?;
        info!("Started input stream: {}", route.from_device);
        route.output_stream.play()?;
        info!("Started output stream: {}", route.to_device);
    }

    info!("Audio routing active with {} routes:", routes.len());
    for route in &routes {
        info!("  {} → {}", route.from_device, route.to_device);
    }

    keep_alive(running, routes, config.audio.keep_alive_sleep_ms);

    info!("Audio routing stopped");
    Ok(())
}

fn validate_routing(config: &Config) -> Result<()> {
    for (route_name, route) in &config.routing {
        if !config.devices.contains_key(&route.from) {
            return Err(anyhow::anyhow!(
                "Route '{}' references unknown source device: '{}'",
                route_name,
                route.from
            ));
        }
        if !config.devices.contains_key(&route.to) {
            return Err(anyhow::anyhow!(
                "Route '{}' references unknown destination device: '{}'",
                route_name,
                route.to
            ));
        }
    }

    let mut seen_routes = HashMap::new();
    for (route_name, route) in &config.routing {
        let key = format!("{}->{}", route.from, route.to);
        if let Some(existing) = seen_routes.get(&key) {
            warn!(
                "Duplicate route detected: '{}' and '{}' both route {} -> {}",
                existing, route_name, route.from, route.to
            );
        }
        seen_routes.insert(key, route_name);
    }

    Ok(())
}

fn handle_input_data(
    data: &[f32],
    producer: &mut HeapProducer<f32>,
    in_channels: u16,
    out_channels: u16,
    gain: f32,
    audio_settings: &AudioSettings,
) {
    if in_channels == 1 && out_channels == 2 {
        for &sample in data {
            if !producer.is_full() {
                let boosted =
                    (sample * gain).clamp(audio_settings.sample_min, audio_settings.sample_max);
                producer.push(boosted).ok();
                producer.push(boosted).ok();
            }
        }
    } else if in_channels == 2 && out_channels == 1 {
        for chunk in data.chunks(2) {
            if chunk.len() == 2 && !producer.is_full() {
                let mixed = ((chunk[0] + chunk[1]) * audio_settings.mix_ratio * gain)
                    .clamp(audio_settings.sample_min, audio_settings.sample_max);
                producer.push(mixed).ok();
            }
        }
    } else {
        for &sample in data {
            if !producer.is_full() {
                let boosted =
                    (sample * gain).clamp(audio_settings.sample_min, audio_settings.sample_max);
                producer.push(boosted).ok();
            }
        }
    }
}

fn keep_alive(running: Arc<AtomicBool>, _routes: Vec<AudioRoute>, sleep_ms: u64) {
    while running.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(sleep_ms));
    }
}
