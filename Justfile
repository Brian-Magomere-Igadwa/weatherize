# Phase 1 Verification
check-core:
    cargo check -p weather-core --no-default-features
    cargo test -p weather-core --all-features

# Phase 2 Firmware Compilation
build-avr:
    cd firmware-avr && cargo +nightly -Zjson-target-spec build --release

# Flash Firmware to Arduino Uno & open console via Ravedude
flash-avr:
    cd firmware-avr && cargo +nightly -Zjson-target-spec run --release

# Phase 3 Commands
check-server:
    cargo check -p weather-actix-server
    cargo test -p weather-actix-server

run-server port="/dev/ttyACM0":
    SERIAL_PORT={{port}} cargo run -p weather-actix-server

# Phase 4 Commands
check-agent:
    cargo check -p cli-agent
    cargo test -p weather-actix-server --test mcp_endpoint

run-agent prompt="How is the room climate right now?":
    cargo run -p cli-agent -- --prompt "{{prompt}}"

# Complete Monorepo Verification
check-all:
    cargo check --workspace --exclude firmware-avr
    cargo test --workspace --exclude firmware-avr
    cd firmware-avr && cargo +nightly -Zjson-target-spec check

# Start LGTM Observability Stack
alloy-up:
    docker compose -f deployment/docker-compose.yml up -d --build

# Stop LGTM Stack
alloy-down:
    docker compose -f deployment/docker-compose.yml down -v

# Run full E2E Integration Suite
e2e-test:
    cargo test -p weather-actix-server --test e2e_pipeline