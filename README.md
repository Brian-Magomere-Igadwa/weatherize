# Kūchō (空調) — End-to-End Climate Telemetry & Agent System

**Kūchō** is an end-to-end, Rust-native IoT climate monitoring platform. It captures environment data directly from physical sensors using bare-metal microcontrollers, streams JSON telemetry over UART to an async backend, exposes Model Context Protocol (MCP) endpoints for local LLM tool usage, and pipelines metrics into an observability stack.

---

## Workspace Architecture

```
kucho-1/
├── Justfile                      # Command runner for build, flash, and stack setup
├── Cargo.toml                    # Workspace manifest & profile overrides
├── rust-toolchain.toml           # Toolchain pinning
├── weather-core/                 # Shared #![no_std] domain logic & JSON protocols
├── firmware-avr/                 # ATmega328P / Arduino Uno bare-metal Rust firmware
├── weather-actix-server/         # Async Actix web server, serial worker, & MCP server
├── cli-agent/                    # Streaming Ollama CLI agent with MCP tool integration
└── deployment/                   # Docker Compose, Grafana, & Grafana Alloy configuration

```

---

## Architecture & Component Breakdown

| Crate / Directory          | Target Environment                   | Key Responsibilities                                                                                                                             |
| -------------------------- | ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| **`weather-core`**         | `#![no_std]` / `std`                 | Domain models (`TelemetryPayload`), serialization protocols, and status classification rules (`OPTIMAL`, `STUFFY`, `HIGH_HUMIDITY`).             |
| **`firmware-avr`**         | Bare-Metal (`avr-atmega328p`)        | Reads DHT11 on pin **D8**, bit-bangs 1-wire protocol, formats `ufmt` JSON, and outputs over 57600 baud UART.                                     |
| **`weather-actix-server`** | Native Async (`tokio` / `actix-web`) | Ingests live UART serial frames, updates thread-safe `tokio::sync::watch` state, serves `/sensor` REST endpoints, and exposes SSE MCP endpoints. |
| **`cli-agent`**            | Native CLI                           | Connects to local Ollama models (`qwen`), executes MCP tool calls (`get_indoor_climate`), and streams agent responses.                           |
| **`deployment/`**          | Docker / LGTM Stack                  | Configures Grafana dashboards, Loki log shipping, and Grafana Alloy telemetry collectors.                                                        |

---

## Hardware Configuration (`firmware-avr`)

- **Microcontroller:** ATmega328P (Arduino Uno)
- **Sensor:** DHT11 Temperature & Humidity Sensor
- **Data Pin:** Digital Pin **D8**
- **Baud Rate:** `57600`
- **Heartbeat LED:** Digital Pin **D13** (toggles on every sampling cycle)

---

## Quickstart Guide

### 1. Hardware Firmware Flash

Plug in your Arduino Uno via USB and flash the release binary using `ravedude`:

```bash
cd firmware-avr
cargo +nightly -Zbuild-std=core,panic_abort -Zjson-target-spec run --release

```

### 2. Start Backend Server

Run the Actix ingestion server to capture serial streams and expose endpoints:

```bash
cargo run -p weather-actix-server

```

### 3. Run Agent CLI

In a separate terminal, trigger the local streaming agent:

```bash
cargo run -p cli-agent

```

### 4. Deploy Observability Stack

Spin up Grafana and Alloy using Docker Compose:

```bash
docker compose -f deployment/docker-compose.yml up -d

```

---

## Testing

Run integration tests for telemetry ingestion and MCP tool endpoint validation:

```bash
cargo test -p weather-actix-server

```
