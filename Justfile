# Show available commands
default:
    @just --list

# ---------------------------------------------------------
# Setup / diagnostics
# ---------------------------------------------------------

doctor:
    ./scripts/doctor.sh

# ---------------------------------------------------------
# Verification
# ---------------------------------------------------------

check-core:
    cargo check -p weather-core --no-default-features
    cargo test -p weather-core --all-features

check-server:
    cargo check -p weather-actix-server
    cargo test -p weather-actix-server

check-agent:
    cargo check -p cli-agent

check-host:
    cargo fmt --check
    cargo check --workspace --exclude firmware-avr
    cargo test --workspace --exclude firmware-avr

check-firmware:
    cd firmware-avr && cargo +nightly -Zjson-target-spec check

check:
    just check-host
    just check-firmware

e2e:
    cargo test -p weather-actix-server --test e2e_pipeline

# ---------------------------------------------------------
# Firmware
# ---------------------------------------------------------

build-firmware:
    cd firmware-avr && cargo +nightly -Zjson-target-spec build --release

flash:
    cd firmware-avr && cargo +nightly -Zjson-target-spec run --release

# ---------------------------------------------------------
# Runtime
# ---------------------------------------------------------

server port="/dev/ttyACM0":
    SERIAL_PORT={{port}} cargo run -p weather-actix-server

agent:
    cargo run -p cli-agent

test-speech text="Oh good. Apparently the vocal cords survived setup.":
    #!/usr/bin/env bash
    set -euo pipefail

    : "${KUCHO_KOKO_BINARY:?KUCHO_KOKO_BINARY is not set}"
    : "${KUCHO_KOKORO_MODEL:?KUCHO_KOKORO_MODEL is not set}"
    : "${KUCHO_KOKORO_VOICES:?KUCHO_KOKORO_VOICES is not set}"

    tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/kucho-speech.XXXXXX")"
    output="$tmp_dir/speech.wav"

    cleanup() {
        rm -rf "$tmp_dir"
    }
    trap cleanup EXIT

    if [[ -n "${ORT_DYLIB_PATH:-}" ]]; then
        export ORT_DYLIB_PATH
    fi

    "$KUCHO_KOKO_BINARY" \
        --model "$KUCHO_KOKORO_MODEL" \
        --data "$KUCHO_KOKORO_VOICES" \
        --style "${KUCHO_VOICE_STYLE:-bm_lewis}" \
        --speed "${KUCHO_SPEECH_SPEED:-0.92}" \
        text "{{text}}" \
        --output "$output"

    if [[ ! -s "$output" ]]; then
        echo "Speech synthesis did not produce a WAV file."
        exit 1
    fi

    if command -v afplay >/dev/null 2>&1; then
        afplay "$output"
    elif command -v aplay >/dev/null 2>&1; then
        aplay "$output"
    else
        echo "No supported audio player found."
        exit 1
    fi

# ---------------------------------------------------------
# Observability
# ---------------------------------------------------------

observability-up:
    docker compose -f deployment/docker-compose.yml up -d

observability-down:
    docker compose -f deployment/docker-compose.yml down
