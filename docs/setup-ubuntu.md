# Kūchō — Ubuntu Setup

This guide takes a fresh Ubuntu machine from a cloned repository to a running
Kūchō installation with live hardware telemetry, local AI, environmental event
detection, and optional speech.

Kūchō runs primarily on the host machine because it interacts directly with
serial hardware, Ollama, and the host audio system.

## What you will need

### Hardware

- Arduino-compatible board used by the Kūchō weather station
- Connected temperature/humidity sensor
- USB cable for the Arduino
- Ubuntu machine

### Software

Kūchō uses:

- Rust
- AVR GCC and `ravedude` for firmware
- Ollama for local LLM inference
- Kokoros for optional local speech synthesis
- `just` for project commands

Speech and the observability stack are optional. You can run the core system
without them.

---

## 1. Install system dependencies

Update your package index:

```bash
sudo apt update
```

Install the base development and runtime dependencies:

```bash
sudo apt install -y \
  build-essential \
  curl \
  git \
  pkg-config \
  wget \
  gcc-avr \
  avr-libc \
  libudev-dev \
  libopus-dev \
  alsa-utils
```

`alsa-utils` provides `aplay`, which Kūchō uses for speech playback on Linux.

---

## 2. Install Rust

Install Rust using rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Reload your shell:

```bash
source "$HOME/.cargo/env"
```

Verify:

```bash
rustc --version
cargo --version
```

Kūchō's host applications use stable Rust.

The AVR firmware requires nightly Rust:

```bash
rustup toolchain install nightly --component rust-src
```

Install `ravedude`:

```bash
cargo install ravedude
```

---

## 3. Install `just`

Kūchō exposes its common development and runtime commands through the root
`Justfile`.

Install it with:

```bash
cargo install just
```

Verify:

```bash
just --version
```

Once the repository is cloned, you can see the available project commands with:

```bash
just --list
```

---

## 4. Clone Kūchō

Clone the repository and enter it:

```bash
git clone <KŪCHŌ_REPOSITORY_URL>
cd <KŪCHŌ_REPOSITORY_DIRECTORY>
```

Check the host workspace:

```bash
just check-host
```

Check the firmware toolchain:

```bash
just check-firmware
```

Or check both:

```bash
just check
```

---

## 5. Run the setup doctor

Kūchō includes a setup diagnostic:

```bash
just doctor
```

The doctor checks the major runtime dependencies, including:

- Rust
- firmware tooling
- Ollama
- required LLM models
- serial-device configuration
- weather-server availability
- telemetry availability
- Kokoros configuration when speech is enabled
- Linux/macOS audio playback support

It is normal for some runtime checks to warn or fail at this point. For example,
Ollama may not be installed yet and the weather server may not be running.

Use the reported fixes as you continue through this guide.

---

## 6. Connect the weather station

Connect the Arduino to the Ubuntu machine over USB.

List likely serial devices:

```bash
ls /dev/ttyACM* /dev/ttyUSB* 2>/dev/null
```

A common Arduino device is:

```text
/dev/ttyACM0
```

Configure the port for the current shell:

```bash
export SERIAL_PORT=/dev/ttyACM0
```

If your device has a different name, use that instead.

### Serial permission errors

Check the device:

```bash
ls -l "$SERIAL_PORT"
```

On Ubuntu, serial devices are commonly accessible through the `dialout` group.

Add your user if necessary:

```bash
sudo usermod -aG dialout "$USER"
```

Log out and back in after changing group membership.

Verify:

```bash
groups
```

---

## 7. Build and flash the firmware

With the Arduino connected:

```bash
just build-firmware
```

Then flash it:

```bash
just flash
```

If flashing fails, check:

```bash
echo "$SERIAL_PORT"
ls -l "$SERIAL_PORT"
```

and rerun:

```bash
just doctor
```

---

## 8. Install Ollama

Install Ollama using its supported Linux installation method.

After installation, verify:

```bash
ollama --version
```

Make sure the Ollama service is running.

Kūchō currently uses two local models:

```bash
ollama pull qwen3:4b
ollama pull qwen2.5:3b
```

The first is used for interactive conversation and MCP tool use.

The second is used for short autonomous environmental commentary.

Verify:

```bash
ollama list
```

Then:

```bash
just doctor
```

The Ollama section should now report the service and both models as available.

---

## 9. Start the weather server

With the Arduino connected and `SERIAL_PORT` configured:

```bash
just server "$SERIAL_PORT"
```

The server receives hardware telemetry and provides the host-side climate
services used by Kūchō.

By default it exposes services on:

```text
http://127.0.0.1:8080
```

In another terminal, verify health:

```bash
curl http://127.0.0.1:8080/health
```

Then verify live telemetry:

```bash
curl http://127.0.0.1:8080/api/v1/telemetry
```

Once readings are arriving, rerun:

```bash
just doctor
```

You should see the weather server and live telemetry checks pass.

---

## 10. Run Kūchō without speech

Speech is optional and is disabled by default.

Make sure Ollama and the weather server are running, then start the agent:

```bash
just agent
```

You should see Kūchō connect to the MCP server and load its available tools.

Try asking:

```text
What's the temperature in here?
```

or:

```text
Has it been getting hotter?
```

The agent can query both current climate conditions and recent climate trends.

Kūchō also listens for significant environmental changes from the weather
server. These events can produce autonomous commentary without requiring a
user question.

At this point, the core system is operational.

---

# Optional: Local speech with Kokoros

Kūchō can synthesize autonomous commentary locally using Kokoros.

Complete the core setup above before adding speech. This makes TTS problems
easier to isolate from weather-server, hardware, or Ollama problems.

## 11. Build Kokoros

Clone Kokoros outside the Kūchō repository:

```bash
cd "$HOME"
git clone https://github.com/lucasjinreal/Kokoros.git
cd Kokoros
```

Download its model and voice assets:

```bash
bash download_all.sh
```

Then build the release binary:

```bash
cargo build --release
```

Verify:

```bash
./target/release/koko --help
```

The expected assets are:

```text
~/Kokoros/target/release/koko
~/Kokoros/checkpoints/kokoro-v1.0.onnx
~/Kokoros/data/voices-v1.0.bin
```

Return to the Kūchō repository.

---

## 12. Configure speech

Export the Kokoros paths:

```bash
export KUCHO_SPEECH=true
export KUCHO_KOKO_BINARY="$HOME/Kokoros/target/release/koko"
export KUCHO_KOKORO_MODEL="$HOME/Kokoros/checkpoints/kokoro-v1.0.onnx"
export KUCHO_KOKORO_VOICES="$HOME/Kokoros/data/voices-v1.0.bin"
```

Kūchō's default voice configuration is:

```bash
export KUCHO_VOICE_STYLE="bm_lewis"
export KUCHO_SPEECH_SPEED="0.92"
```

`bm_lewis` is the British male voice currently used by Kūchō.

On a normal Ubuntu setup, `ORT_DYLIB_PATH` should not need to be configured.
Do not copy a macOS ONNX Runtime path into the Ubuntu configuration.

---

## 13. Sanity-check speech

Before involving the agent, test the TTS layer independently:

```bash
just test-speech
```

You should hear the test sentence through the machine's default audio output.

This verifies the complete local speech path:

```text
Kokoros binary
    ↓
Kokoro model + voices
    ↓
WAV generation
    ↓
aplay
    ↓
speakers
```

If it fails, run:

```bash
just doctor
```

and fix the Speech section before continuing.

---

## 14. Run Kūchō with speech

With the environment variables still configured:

```bash
just agent
```

When the environmental monitor detects a significant change, Kūchō can now:

1. receive the event from the weather server;
2. generate short local commentary with Ollama;
3. display the commentary in the CLI;
4. synthesize it with Kokoros;
5. play it through the host audio system.

Speech runs independently of the interactive prompt so audio generation does
not block normal agent interaction.

---

## 15. Final sanity check

With the hardware connected and the runtime services available:

```bash
just doctor
```

Then verify the project:

```bash
just check
```

A fully configured installation should have:

```text
Arduino firmware       ✓
Serial telemetry       ✓
Weather server         ✓
Telemetry history      ✓
Environment detection  ✓
MCP tools              ✓
SSE events             ✓
Ollama                  ✓
Kūchō agent             ✓
Kokoros speech          ✓  optional
```

The usual runtime layout is:

```text
Terminal 1
  Ollama

Terminal 2
  just server "$SERIAL_PORT"

Terminal 3
  just agent
```

---

# Configuration reference

The repository contains `.env.example` documenting the supported environment
variables.

Important server variables:

```text
SERIAL_PORT
BAUD_RATE
HOST
PORT
```

Important agent variables:

```text
KUCHO_MCP_URL
KUCHO_OLLAMA_URL
KUCHO_CHAT_MODEL
KUCHO_COMMENTARY_MODEL
```

Speech variables:

```text
KUCHO_SPEECH
KUCHO_KOKO_BINARY
KUCHO_KOKORO_MODEL
KUCHO_KOKORO_VOICES
KUCHO_VOICE_STYLE
KUCHO_SPEECH_SPEED
ORT_DYLIB_PATH
```

CLI flags may also be used for agent configuration. Explicit CLI configuration
takes precedence over defaults.

---

# Useful commands

List available commands:

```bash
just --list
```

Diagnose the machine:

```bash
just doctor
```

Verify the codebase:

```bash
just check
```

Build firmware:

```bash
just build-firmware
```

Flash firmware:

```bash
just flash
```

Start the weather server:

```bash
just server "$SERIAL_PORT"
```

Start Kūchō:

```bash
just agent
```

Test speech independently:

```bash
just test-speech
```

---

# Troubleshooting

## No serial device appears

Check:

```bash
ls /dev/ttyACM* /dev/ttyUSB* 2>/dev/null
```

Disconnect and reconnect the Arduino and run the command again.

Also check:

```bash
dmesg | tail -n 30
```

## Permission denied opening the serial port

Check:

```bash
groups
ls -l "$SERIAL_PORT"
```

If necessary:

```bash
sudo usermod -aG dialout "$USER"
```

Then log out and back in.

## Weather server starts but telemetry is unavailable

Check:

```bash
echo "$SERIAL_PORT"
curl http://127.0.0.1:8080/health
curl http://127.0.0.1:8080/api/v1/telemetry
```

The server can be healthy while the hardware telemetry source is unavailable.

## Agent cannot connect to MCP

Verify:

```bash
curl http://127.0.0.1:8080/health
```

and check:

```bash
echo "$KUCHO_MCP_URL"
```

The default MCP endpoint is:

```text
http://127.0.0.1:8080/mcp
```

## Ollama is unavailable

Check:

```bash
ollama list
```

and:

```bash
curl http://127.0.0.1:11434/api/tags
```

Then rerun:

```bash
just doctor
```

## Speech does not work

First isolate speech from the rest of Kūchō:

```bash
just test-speech
```

Then:

```bash
just doctor
```

Check that these are configured:

```bash
echo "$KUCHO_KOKO_BINARY"
echo "$KUCHO_KOKORO_MODEL"
echo "$KUCHO_KOKORO_VOICES"
```

Also verify Linux audio playback:

```bash
command -v aplay
```

## Speech is disabled

This is the default.

Enable it with:

```bash
export KUCHO_SPEECH=true
```

and configure the three required Kokoros paths before starting the agent.

---

## Next steps

For an explanation of how the firmware, weather server, MCP layer,
environmental monitor, agent, Ollama, and speech pipeline fit together, see
`docs/architecture.md`.

For macOS-specific setup, see `docs/setup-macos.md`.

For additional failure modes and diagnostic steps, see
[`troubleshooting.md`](troubleshooting.md).