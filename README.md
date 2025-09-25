# Audio Router

A Windows service that provides dynamic audio routing between different audio devices with a flexible, configuration-driven approach.

## Why
I've used a GoXLR for my personal audio needs for a while now, and recently I got a wireless headset and wanted to use it with the GoXLR. There's a lot of programs that lets you achieve this, however, most of them are either paid, has a ton of extra features I don't need, or are just overly complicated.

This app lets me connect my GoXLR headphones output to my PC's AUX In, and my PC's AUX Out to my GoXLR's Line In, effectively routing all audio from my PC to my headset, and my microphone to the GoXLR. This conveniently allows me to swap easily between the headset mic and the GoXLR mic, without switching inputs on the chat app.

You could probably use this for a range of other use cases as well, I just wanted to explain my personal use case and why this exists. The included config is the one I use personally.

## Why not
If you need literally anything but audio routing, there's a lot of other apps that'll do this and more. I've spent some time making this fast, and I've been unable to break below the current latency. It's fast enough that you won't notice it in most scenarios, but if you use the mic monitor feature on the GoXLR, you will notice it. That's worth keeping in mind.

## Prebuilt Binary Installation
I keep the latest release binaries on the releases page, this includes the executable and a sample config file. You can extract it to a location of your choice, edit the config file, and run the install command to install it as a service (Requires administrator privileges). This will automatically start the service, and it will also be configured to start on boot.

## Usage

### Console Mode
```cmd
audio_router.exe
audio_router.exe console
```

### Windows Service

**Install service (run as administrator):**
```cmd
audio_router.exe install
```

**Start/Stop service (Windows Command Prompt as administrator):**
```cmd
sc start AudioService
sc stop AudioService  
```

**Start/Stop service (PowerShell as administrator):**
```powershell
Start-Service AudioService
Stop-Service AudioService
```

**Or use Services management console:**
- Run `services.msc`
- Find "Audio Router Service"
- Right-click to start/stop

**Uninstall service:**
```cmd
audio_router.exe uninstall
```

### Utilities
```cmd
# List available audio devices
audio_router.exe list-devices
```

## Logging

Logs are written to `logs.txt` next to the executable. The log file is cleared on each startup.

### Configuration Details

#### Device Configuration
- **name**: Exact or partial device name (use `list-devices` to find names)
- **type**: Must be either `input` or `output`
- **buffer_size**: Audio stream buffer size for this device
- **primary_buffer**: Ring buffer size for audio routing
- **gain**: Audio gain multiplier for this device (1.0 = no gain)
- Device aliases (keys) can be any descriptive name

#### Routing Configuration
- **from**: Source device alias (must be an input device)
- **to**: Destination device alias (must be an output device)
- Route names can be any descriptive identifier
- Multiple routes are supported
- Each route uses the input device's buffer and gain settings

#### Global Audio Settings
- **prefill_samples**: Pre-fill buffer with silence samples
- **keep_alive_sleep_ms**: Main loop sleep duration in milliseconds
- **stereo_to_mono_mix_ratio**: Mixing ratio for stereo to mono conversion (0.5 = average both channels)
- **audio_sample_min/max**: Audio sample clamp bounds

## Example Configurations
```yaml
# Audio Routing Configuration

# Device definitions with type (input/output)
devices:
  line_in:
    name: "Line In (Realtek(R) Audio)"
    type: input
    buffer_size: 8
    primary_buffer: 960
    gain: 1.0
  line_out:
    name: "Speakers (Realtek(R) Audio)"
    type: output
    buffer_size: 8
    primary_buffer: 960
    gain: 1.0
  headset:
    name: "Headphones (Chat-Audeze Maxwell)"
    type: output
    buffer_size: 8
    primary_buffer: 960
    gain: 1.0
  mic:
    name: "Microphone (Chat-Audeze Maxwell)"
    type: input
    buffer_size: 8
    primary_buffer: 960
    gain: 1.0

# Routing configuration
routing:
  line_in_to_headset:
    from: "line_in"
    to: "headset"
  mic_to_line_out:
    from: "mic"
    to: "line_out"

# Audio settings
audio:
  # Pre-fill buffer with silence (samples)
  prefill_samples: 4800

  # Audio processing constants
  # Keep-alive loop sleep duration (milliseconds)
  keep_alive_sleep_ms: 100

  # Ratio for mixing stereo to mono (0.5 = average both channels)
  stereo_to_mono_mix_ratio: 0.5

  # Audio sample clamp bounds
  audio_sample_min: -1.0
  audio_sample_max: 1.0

# Logging settings
logging:
  # Log level: trace, debug, info, warn, error
  level: info

# Device wait settings (for service mode)
device_wait:
  # Enable waiting for devices to become available
  enabled: true

  # Maximum time to wait for devices (seconds)
  max_wait_time: 60

  # Time between device check attempts (seconds)
  retry_interval: 2

  # Continue even if some devices are not found
  allow_partial: false
```