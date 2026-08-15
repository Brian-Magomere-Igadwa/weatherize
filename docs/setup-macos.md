# Kūchō — macOS Setup

This guide covers the macOS-specific setup required to run Kūchō.

For an overview of the complete setup and runtime flow, see
[`setup-ubuntu.md`](setup-ubuntu.md). The overall architecture is the same on
macOS:

```text
Arduino
   ↓ USB serial
weather-actix-server
   ↓ MCP + SSE
cli-agent
   ├── Ollama
   └── Kokoros → afplay → speakers
```

The main macOS differences are:

- Homebrew is used for system dependencies.
- Arduino serial devices normally appear under `/dev/cu.*`.
- Kūchō uses `afplay` for audio playback.
- Intel Macs may require an explicit ONNX Runtime dynamic library for Kokoros.

---

## 1. Install Homebrew

If Homebrew is not already installed, install it from the official Homebrew
installation instructions.

Verify:

```bash
brew --version
```

---

## 2. Install system dependencies

Install the development tools used by Kūchō:

```bash
brew install \
  just \
  pkg-config \
  wget \
  libusb \
  libserialport \
  avr-gcc
```

macOS already provides `afplay`, which Kūchō uses for speech playback.

Verify:

```bash
command -v afplay
```

---

## 3. Install Rust

Install Rust with rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Reload the shell:

```bash
source "$HOME/.cargo/env"
```

Verify:

```bash
rustc --version
cargo --version
```

Install the nightly toolchain required by the AVR firmware:

```bash
rustup toolchain install nightly --component rust-src
```

Install `ravedude`:

```bash
cargo install ravedude
```

---

## 4. Clone and check Kūchō

Clone the repository and enter it:

```bash
git clone <KŪCHŌ_REPOSITORY_URL>
cd <KŪCHŌ_REPOSITORY_DIRECTORY>
```

List the available project commands:

```bash
just --list
```

Run the setup doctor:

```bash
just doctor
```

At this stage it is normal for checks involving Ollama, the Arduino, the
weather server, or speech to report warnings or failures.

---

## 5. Find the Arduino serial device

Connect the Arduino over USB.

List likely devices:

```bash
ls /dev/cu.*
```

Arduino devices commonly look similar to:

```text
/dev/cu.usbmodem1101
```

or:

```text
/dev/cu.usbserial-*
```

Set the device for the current shell:

```bash
export SERIAL_PORT=/dev/cu.usbmodem1101
```

Use the actual device reported by your machine.

Verify:

```bash
echo "$SERIAL_PORT"
ls -l "$SERIAL_PORT"
```

Unlike Ubuntu, macOS does not normally require adding your user to a `dialout`
group.

---

## 6. Build and flash the firmware

Check the firmware toolchain:

```bash
just check-firmware
```

Build:

```bash
just build-firmware
```

Flash the connected board:

```bash
just flash
```

If flashing fails, verify `SERIAL_PORT` and rerun:

```bash
just doctor
```

---

## 7. Install Ollama

Install Ollama for macOS.

Kūchō currently uses:

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

The Ollama checks should report both models as available.

---

## 8. Start the weather server

With `SERIAL_PORT` configured:

```bash
just server "$SERIAL_PORT"
```

In another terminal:

```bash
curl http://127.0.0.1:8080/health
```

Then:

```bash
curl http://127.0.0.1:8080/api/v1/telemetry
```

Once live telemetry is available, the hardware-to-server path is working.

---

## 9. Run Kūchō without speech

Speech is optional and disabled by default.

With Ollama and the weather server running:

```bash
just agent
```

Try:

```text
What's the temperature in here?
```

or:

```text
Has it been getting hotter?
```

At this point the core Kūchō system should work without Kokoros.

---

# Optional: Local speech

## 10. Build Kokoros

Clone Kokoros outside the Kūchō repository:

```bash
cd "$HOME"
git clone https://github.com/lucasjinreal/Kokoros.git
cd Kokoros
```

Download its assets:

```bash
bash download_all.sh
```

Build:

```bash
cargo build --release
```

Verify:

```bash
./target/release/koko --help
```

Expected files:

```text
~/Kokoros/target/release/koko
~/Kokoros/checkpoints/kokoro-v1.0.onnx
~/Kokoros/data/voices-v1.0.bin
```

---

## 11. Configure speech

Return to the Kūchō repository and export:

```bash
export KUCHO_SPEECH=true
export KUCHO_KOKO_BINARY="$HOME/Kokoros/target/release/koko"
export KUCHO_KOKORO_MODEL="$HOME/Kokoros/checkpoints/kokoro-v1.0.onnx"
export KUCHO_KOKORO_VOICES="$HOME/Kokoros/data/voices-v1.0.bin"

export KUCHO_VOICE_STYLE="bm_lewis"
export KUCHO_SPEECH_SPEED="0.92"
```

Then:

```bash
just doctor
```

If Kokoros works normally on your Mac, continue to the speech test.

If you are using an Intel Mac and encounter an ONNX Runtime error, continue to
the Intel-specific section below.

---

# Intel macOS: ONNX Runtime workaround

This section is only required if Kokoros fails with an error similar to:

```text
Failed to load ONNX Runtime dylib
```

or reports that the ONNX Runtime binary is incompatible with the version
expected by the Rust `ort` crate.

This was required during development on an x86_64 Intel Mac.

## 12. Configure Kokoros for dynamic ONNX Runtime loading

In the Kokoros repository, open:

```text
kokoros/Cargo.toml
```

If the `ort` dependency is configured with its normal prebuilt/default
features, change it to use dynamic loading.

For the version used during Kūchō development:

```toml
ort = {
    version = "2.0.0-rc.11",
    default-features = false,
    features = ["std", "load-dynamic"]
}
```

Rebuild Kokoros:

```bash
cargo build --release
```

---

## 13. Install a compatible x86_64 ONNX Runtime

The `ort` version used during development required ONNX Runtime `1.23.x` or
newer.

Download an x86_64 macOS ONNX Runtime release compatible with the installed
`ort` version and extract it somewhere stable.

For example:

```text
~/onnxruntime-osx-x86_64-1.23.2/
```

Verify that the dynamic library exists:

```bash
find ~/onnxruntime-osx-x86_64-1.23.2 \
  -name "libonnxruntime*.dylib" \
  -print
```

Then configure the runtime library:

```bash
export ORT_DYLIB_PATH="$HOME/onnxruntime-osx-x86_64-1.23.2/lib/libonnxruntime.dylib"
```

Do not set `ORT_DYLIB_PATH` to an older incompatible ONNX Runtime.

For example, ONNX Runtime `1.20.x` is too old for an `ort` build that explicitly
requires `>= 1.23.x`.

---

## 14. Test Kokoros independently

Before involving Kūchō:

```bash
cd "$HOME/Kokoros"

mkdir -p tmp

ORT_DYLIB_PATH="$ORT_DYLIB_PATH" \
./target/release/koko \
  --style "bm_lewis" \
  --speed 0.92 \
  text \
  "Oh good... apparently the runtime library finally decided to cooperate." \
  --output tmp/kucho-test.wav
```

Play it:

```bash
afplay tmp/kucho-test.wav
```

If you hear speech, the Kokoros + ONNX Runtime layer is working.

Return to Kūchō and run:

```bash
just test-speech
```

This tests the same speech path through Kūchō's project tooling.

---

## 15. Final sanity check

Run:

```bash
just doctor
```

Then:

```bash
just check
```

For speech:

```bash
just test-speech
```

Finally:

```bash
just agent
```

The normal runtime layout is:

```text
Terminal 1
  Ollama

Terminal 2
  just server "$SERIAL_PORT"

Terminal 3
  just agent
```

---

# Configuration persistence

The `export` commands in this guide apply only to the current shell session.

During initial setup, this is intentional: it makes configuration explicit and
easy to troubleshoot.

Once your installation works, you can persist the appropriate values in your
shell configuration or another local environment-loading mechanism.

Do not commit machine-specific paths or local secrets to the repository.

See the repository's `.env.example` for the supported configuration variables.

---

# Troubleshooting

## `wget: command not found`

Kokoros uses `wget` in its model/voice download scripts.

Install it:

```bash
brew install wget
```

Then rerun:

```bash
bash download_all.sh
```

## `Failed to load ONNX Runtime dylib`

If you are on an Intel Mac, follow the ONNX Runtime workaround above.

Check:

```bash
echo "$ORT_DYLIB_PATH"
```

and:

```bash
ls -l "$ORT_DYLIB_PATH"
```

## ONNX Runtime version mismatch

An error such as:

```text
expected version >= '1.23.x', but got '1.20.1'
```

means the dynamic library is being found successfully but is too old for the
installed `ort` crate.

Install a compatible ONNX Runtime version and update `ORT_DYLIB_PATH`.

## Kūchō generates commentary but no sound plays

First run:

```bash
just test-speech
```

Then:

```bash
just doctor
```

Verify:

```bash
command -v afplay
```

and check the configured Kokoros paths.

## Arduino device is not `/dev/ttyACM0`

That is expected on macOS.

Find the actual device:

```bash
ls /dev/cu.*
```

Then:

```bash
export SERIAL_PORT=/dev/cu.usbmodem...
```

---

## Next steps

For the complete system design, see [`architecture.md`](architecture.md).

For the canonical Linux setup, see [`setup-ubuntu.md`](setup-ubuntu.md).

For additional failure modes and diagnostic steps, see
[`troubleshooting.md`](troubleshooting.md).
