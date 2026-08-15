# Kūchō Troubleshooting

This guide covers common setup and runtime failures across Kūchō's hardware,
server, Ollama, agent, and speech layers.

Start with the setup doctor:

```bash
just doctor
```

For a complete project verification:

```bash
just check
```

The easiest way to debug Kūchō is to work through the system one boundary at a
time:

```text
Arduino
   ↓
serial telemetry
   ↓
weather-actix-server
   ↓
REST / MCP / SSE
   ↓
cli-agent
   ↓
Ollama
   ↓
Kokoros
   ↓
audio
```

Do not debug the entire pipeline at once. Find the first boundary that is not
working.

---

# Setup doctor

Run:

```bash
just doctor
```

The doctor checks the major local dependencies and runtime configuration.

A warning does not necessarily mean Kūchō cannot run.

For example:

```text
[!] Weather server is not currently running
```

is expected if you run the doctor before starting the server.

A failed requirement should include enough information to identify the missing
dependency or configuration.

---

# Arduino and serial connection

## No serial device appears

### Ubuntu

Check:

```bash
ls /dev/ttyACM* /dev/ttyUSB* 2>/dev/null
```

### macOS

Check:

```bash
ls /dev/cu.*
```

Disconnect and reconnect the Arduino and compare the output.

You can also inspect recent device messages on Linux:

```bash
dmesg | tail -n 30
```

Once identified:

```bash
export SERIAL_PORT=<your-device>
```

Verify:

```bash
echo "$SERIAL_PORT"
ls -l "$SERIAL_PORT"
```

---

## `/dev/ttyACM0` does not exist

`/dev/ttyACM0` is only a convenient Linux default.

Your actual serial device may be:

```text
/dev/ttyACM1
/dev/ttyUSB0
/dev/cu.usbmodem1101
/dev/cu.usbserial-...
```

Find the correct device and set:

```bash
export SERIAL_PORT=<actual-device>
```

Then rerun:

```bash
just doctor
```

---

## Permission denied opening the serial port on Ubuntu

Inspect the device:

```bash
ls -l "$SERIAL_PORT"
```

Check your groups:

```bash
groups
```

Ubuntu commonly grants serial-device access through the `dialout` group.

Add your user:

```bash
sudo usermod -aG dialout "$USER"
```

Log out and back in before trying again.

---

# Firmware

## AVR firmware does not compile

Check the firmware environment:

```bash
just check-firmware
```

Verify nightly Rust:

```bash
rustup toolchain list
```

If missing:

```bash
rustup toolchain install nightly --component rust-src
```

Check AVR GCC:

```bash
avr-gcc --version
```

Then:

```bash
just build-firmware
```

---

## `ravedude` not found

Install it:

```bash
cargo install ravedude
```

Verify:

```bash
ravedude --version
```

Then retry:

```bash
just flash
```

---

## Firmware builds but flashing fails

First verify the board:

```bash
echo "$SERIAL_PORT"
ls -l "$SERIAL_PORT"
```

Then:

```bash
just doctor
```

If the serial device changed after reconnecting the Arduino, update
`SERIAL_PORT`.

---

# Weather server

## Server does not start

Run it directly through the project command:

```bash
just server "$SERIAL_PORT"
```

Verify that the configured serial port exists:

```bash
ls -l "$SERIAL_PORT"
```

Also check whether port `8080` is already occupied.

### Linux

```bash
ss -ltnp | grep ':8080'
```

### macOS

```bash
lsof -i :8080
```

---

## Server is healthy but telemetry is unavailable

Check:

```bash
curl http://127.0.0.1:8080/health
```

Then:

```bash
curl http://127.0.0.1:8080/api/v1/telemetry
```

These are different checks.

A healthy server means the HTTP application is running.

It does **not** necessarily mean the Arduino is connected or valid telemetry
has been received.

If health succeeds but telemetry does not:

1. verify the Arduino is connected;
2. verify `SERIAL_PORT`;
3. verify the firmware is running;
4. verify the configured baud rate;
5. inspect the server logs.

Then rerun:

```bash
just doctor
```

---

# MCP

## Agent cannot connect to MCP

Check the server first:

```bash
curl http://127.0.0.1:8080/health
```

Check the configured MCP URL:

```bash
echo "${KUCHO_MCP_URL:-http://127.0.0.1:8080/mcp}"
```

The default is:

```text
http://127.0.0.1:8080/mcp
```

Restart the server if necessary:

```bash
just server "$SERIAL_PORT"
```

Then start the agent:

```bash
just agent
```

---

## MCP works but current climate is unavailable

The MCP server can be running before the first sensor observation arrives.

Verify telemetry independently:

```bash
curl http://127.0.0.1:8080/api/v1/telemetry
```

If telemetry is unavailable, debug the serial/hardware path rather than the
agent.

---

## Climate trend is unavailable

Trend analysis requires telemetry history.

Immediately after starting the server there may not yet be enough observations
for the requested window.

Allow telemetry to accumulate and try again.

For example, a request about the last five minutes cannot contain a meaningful
five-minute history immediately after startup.

---

# Environmental events

## No autonomous events appear

First verify that ordinary telemetry is arriving:

```bash
curl http://127.0.0.1:8080/api/v1/telemetry
```

Environmental events are not emitted for every sensor reading.

The deterministic monitor emits events only when configured conditions are
met, such as:

- sufficiently rapid temperature changes;
- sufficiently rapid humidity changes;
- relevant safety-status transitions.

Cooldown and hysteresis logic can also intentionally suppress events.

No event therefore does not automatically mean the event system is broken.

---

## Events appear repeatedly

Kūchō uses cooldowns and hysteresis to reduce repeated notifications.

If events appear more frequently than expected, run the monitor tests:

```bash
cargo test -p weather-actix-server monitor
```

Also verify that monitor state is being preserved rather than recreated for
every observation.

---

# Ollama

## `ollama` not found

Install Ollama for your operating system and verify:

```bash
ollama --version
```

Then:

```bash
just doctor
```

---

## Ollama is installed but unreachable

Check:

```bash
curl http://127.0.0.1:11434/api/tags
```

If this fails, ensure the Ollama service/application is running.

Check your configured URL:

```bash
echo "${KUCHO_OLLAMA_URL:-http://127.0.0.1:11434}"
```

---

## Required model missing

Kūchō currently uses:

```text
qwen3:4b
qwen2.5:3b
```

Install them:

```bash
ollama pull qwen3:4b
ollama pull qwen2.5:3b
```

Verify:

```bash
ollama list
```

Then:

```bash
just doctor
```

---

## Commentary takes a long time

Autonomous commentary is generated locally through Ollama.

Generation latency depends on:

- model size;
- CPU/GPU performance;
- available memory;
- whether the model is already loaded;
- other local inference workloads.

Test Ollama independently if necessary.

A slow model response is different from a failed environmental event.

If you see the event before commentary generation begins, the
hardware/server/event pipeline has already succeeded.

---

# Speech

Always debug speech independently before debugging it through the agent.

Run:

```bash
just test-speech
```

If that succeeds, the Kokoros → WAV → audio path is operational.

---

## Speech is disabled

Speech is optional and disabled by default.

Check:

```bash
echo "${KUCHO_SPEECH:-false}"
```

Enable it:

```bash
export KUCHO_SPEECH=true
```

Then configure the required Kokoros paths.

---

## Kokoros paths are missing

Check:

```bash
echo "$KUCHO_KOKO_BINARY"
echo "$KUCHO_KOKORO_MODEL"
echo "$KUCHO_KOKORO_VOICES"
```

Example:

```bash
export KUCHO_KOKO_BINARY="$HOME/Kokoros/target/release/koko"
export KUCHO_KOKORO_MODEL="$HOME/Kokoros/checkpoints/kokoro-v1.0.onnx"
export KUCHO_KOKORO_VOICES="$HOME/Kokoros/data/voices-v1.0.bin"
```

Then:

```bash
just doctor
```

---

## `wget: command not found` while setting up Kokoros

Kokoros uses `wget` in its download scripts.

### Ubuntu

```bash
sudo apt install wget
```

### macOS

```bash
brew install wget
```

Then rerun:

```bash
bash download_all.sh
```

---

## `Failed to load ONNX Runtime dylib`

This is particularly relevant to Intel macOS.

Check:

```bash
echo "$ORT_DYLIB_PATH"
```

Then:

```bash
ls -l "$ORT_DYLIB_PATH"
```

If the file does not exist, correct the configured path.

If Kokoros was built using `ort` dynamic loading, it needs a compatible ONNX
Runtime dynamic library at runtime.

See `setup-macos.md` for the Intel macOS setup.

---

## ONNX Runtime version mismatch

An error similar to:

```text
expected version >= '1.23.x', but got '1.20.1'
```

means dynamic loading itself is working.

The problem is that the discovered ONNX Runtime library is older than the
version expected by the installed Rust `ort` crate.

Use a compatible runtime and update:

```bash
export ORT_DYLIB_PATH=<compatible-libonnxruntime.dylib>
```

Then retry:

```bash
just test-speech
```

---

## Kokoros generates audio but nothing is heard

Check the platform audio command.

### macOS

```bash
command -v afplay
```

### Ubuntu

```bash
command -v aplay
```

Ubuntu provides `aplay` through:

```bash
sudo apt install alsa-utils
```

Then:

```bash
just test-speech
```

---

## `AudioFileOpen failed`

First avoid debugging through the full agent.

Run:

```bash
just test-speech
```

Verify that Kokoros actually creates a non-empty WAV file before the audio
player is invoked.

This error can occur when the audio player is given an invalid, missing, or
empty output file rather than because audio playback itself is broken.

---

# CLI behavior

## Autonomous output appears while typing

The CLI currently has two asynchronous sources of terminal output:

```text
interactive conversation
autonomous environmental events
```

An environmental event can therefore be printed while the user is entering a
question.

This is a known v0.1 limitation rather than an environmental-monitoring
failure.

A future prompt-aware terminal UI can redraw user input safely around
asynchronous messages.

---

## Two announcements speak at once

Speech playback is non-blocking, but v0.1 does not yet serialize all
announcements through a dedicated speech queue.

Events arriving close together can therefore start overlapping speech jobs.

This is a known limitation.

---

# Observability

The Docker-based observability stack is optional.

It is not required to run:

- hardware telemetry;
- the weather server;
- MCP;
- the agent;
- Ollama;
- speech.

If observability is broken, first verify the core system without it:

```bash
just doctor
just server "$SERIAL_PORT"
just agent
```

Only debug the observability stack after the primary runtime is working.

---

# Full diagnostic sequence

When you do not know where the problem is, use this order.

## 1. Machine

```bash
just doctor
```

## 2. Codebase

```bash
just check
```

## 3. Hardware

```bash
echo "$SERIAL_PORT"
ls -l "$SERIAL_PORT"
```

## 4. Server

```bash
curl http://127.0.0.1:8080/health
```

## 5. Telemetry

```bash
curl http://127.0.0.1:8080/api/v1/telemetry
```

## 6. Ollama

```bash
curl http://127.0.0.1:11434/api/tags
```

## 7. Agent

```bash
just agent
```

## 8. Speech

```bash
just test-speech
```

The first failing stage is usually the layer that should be investigated.

---

# Still stuck?

Collect the output of:

```bash
just doctor
```

along with:

```bash
rustc --version
cargo --version
uname -a
```

and the error output from the failing command.

For hardware issues, also include:

```bash
echo "$SERIAL_PORT"
ls -l "$SERIAL_PORT"
```

This usually provides enough information to identify which Kūchō subsystem is
failing.
