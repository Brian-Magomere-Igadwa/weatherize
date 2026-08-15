# Kūchō

**A local-first ambient AI assistant that can sense the room, understand environmental changes, and complain about them out loud.**

![Kūchō weather station](docs/images/kucho-weather-station.jpg)

Kūchō connects physical environmental sensors to a local AI agent. It continuously observes indoor temperature and humidity, detects meaningful changes using deterministic Rust logic, exposes live and historical climate data through MCP, and can autonomously comment on changes using local LLMs and speech synthesis.

```text
"The room's gone from comfortable to stuffy."

"Oh joy, humidity's taking a wild ride."

"Apparently opening a window was too ambitious."
```

No cloud AI API is required for the core runtime.

---

## What it does

Kūchō combines:

- **Physical sensing** — an Arduino-based weather station provides live temperature and humidity telemetry.
- **Telemetry history** — recent observations are retained for trend analysis.
- **Deterministic monitoring** — thresholds, cooldowns, and hysteresis detect meaningful environmental changes without relying on an LLM.
- **MCP tools** — the agent can query current conditions and recent climate trends.
- **Autonomous events** — significant changes are pushed to the agent over Server-Sent Events.
- **Local AI** — Ollama handles interactive conversation and autonomous commentary.
- **Local speech** — optional Kokoros TTS gives Kūchō a voice.
- **Observability** — optional infrastructure can be run separately from the core hardware/agent loop.

The important boundary is simple:

> **Rust decides that something happened. The LLM decides how to say it.**

---

## Example

Ask Kūchō about the room:

```text
> is it getting hotter or what?

[Kūchō is working...]

[Kūchō Agent: Calling MCP Tool 'get_climate_trend']

The current indoor climate is stable — neither warming nor cooling.
Temperature remains at 23.0°C and humidity is holding around 51%.
```

But Kūchō does not need to wait for a question.

When the deterministic monitor detects a meaningful environmental change:

```text
[Kūchō noticed something...]

Oh joy, humidity's taking a wild ride! It jumped up by 10%.

[Kūchō would speak]:
Oh joy, humidity's taking a wild ride! It jumped up by 10%.
```

With speech enabled, the final observation is synthesized and played through
the host audio system.

---

## Architecture

```text
                    ┌─────────────────┐
                    │     Arduino     │
                    │    Sensors      │
                    └────────┬────────┘
                             │
                         USB serial
                             │
                             ▼
              ┌────────────────────────────┐
              │    weather-actix-server    │
              │                            │
              │  telemetry → history       │
              │               │            │
              │               ├─ trends    │
              │               │            │
              │               ▼            │
              │       environment monitor  │
              │                            │
              │      MCP          SSE      │
              └───────┬───────────┬────────┘
                      │           │
                      ▼           ▼
                   ┌────────────────┐
                   │   cli-agent    │
                   └───────┬────────┘
                           │
                    ┌──────┴──────┐
                    │             │
                    ▼             ▼
                 Ollama        Kokoros
                                  │
                                  ▼
                               speakers
```

There are two server-to-agent paths:

```text
MCP
└── request-driven
    └── "What's happening?"

SSE
└── event-driven
    └── "Something happened."
```

For the full system design, see
[`docs/architecture.md`](docs/architecture.md).

---

## Workspace

Kūchō is a Rust workspace composed of four main crates:

```text
weather-core/
├── shared environmental domain types and events

firmware-avr/
├── microcontroller firmware and sensor telemetry

weather-actix-server/
├── serial ingestion
├── telemetry history
├── trend analysis
├── environmental monitoring
├── REST API
├── MCP server
└── SSE event stream

cli-agent/
├── interactive agent
├── MCP tool use
├── SSE event consumption
├── Ollama integration
└── optional speech
```

---

## Quick start

### 1. Clone the repository

```bash
git clone <KŪCHŌ_REPOSITORY_URL>
cd <KŪCHŌ_REPOSITORY_DIRECTORY>
```

### 2. Diagnose your setup

Kūchō includes a setup doctor:

```bash
just doctor
```

It checks the major local requirements and tells you which parts of the stack
still need configuration.

### 3. Verify the workspace

```bash
just check
```

### 4. Follow the setup guide for your OS

**Ubuntu**

[`docs/setup-ubuntu.md`](docs/setup-ubuntu.md)

**macOS**

[`docs/setup-macos.md`](docs/setup-macos.md)

The core agent can run without speech. Kokoros should be configured only after
the hardware, server, and Ollama paths are working.

---

## Running Kūchō

A normal development session uses three local processes.

### Ollama

Make sure Ollama is running with the required models installed:

```bash
ollama pull qwen3:4b
ollama pull qwen2.5:3b
```

### Weather server

Connect the weather station and configure its serial device:

```bash
export SERIAL_PORT=<your-serial-device>
```

Then:

```bash
just server "$SERIAL_PORT"
```

### Agent

In another terminal:

```bash
just agent
```

Then ask something:

```text
What's the temperature in here?

Has the humidity been rising?

Is it getting hotter?
```

Or leave it alone and wait for Kūchō to notice something itself.

For platform-specific installation details, use the setup guides rather than
copying machine-specific paths from another installation.

---

## Speech

Speech is optional.

When enabled, autonomous commentary follows this path:

```text
EnvironmentEvent
      ↓
Ollama commentary model
      ↓
natural-language observation
      ↓
Kokoros
      ↓
WAV
      ↓
afplay / aplay
      ↓
speakers
```

The current default voice is the British male Kokoro voice:

```text
bm_lewis
```

Once speech is configured, test it independently:

```bash
just test-speech
```

This deliberately isolates TTS from the rest of the agent so speech problems
can be diagnosed without involving MCP, SSE, or environmental monitoring.

See the OS setup guides for Kokoros installation and configuration.

---

## Useful commands

```bash
# Show available project commands
just --list

# Diagnose local setup
just doctor

# Check the workspace
just check

# Check firmware tooling
just check-firmware

# Build firmware
just build-firmware

# Flash the Arduino
just flash

# Start the weather server
just server "$SERIAL_PORT"

# Start the agent
just agent

# Test Kokoros speech independently
just test-speech

# Start optional observability infrastructure
just observability-up

# Stop optional observability infrastructure
just observability-down
```

---

## API and agent interfaces

The weather server exposes several boundaries used by the rest of the system.

### Health

```text
GET /health
```

### Current telemetry

```text
GET /api/v1/telemetry
```

### Environmental event stream

```text
GET /api/v1/events
```

Events are delivered using Server-Sent Events.

### MCP

```text
POST /mcp
```

Current MCP tools include:

```text
get_indoor_climate
get_climate_trend
```

The agent discovers the available tool definitions from the server rather than
hard-coding physical sensor data into the LLM.

---

## Why deterministic monitoring?

Kūchō does not send every sensor observation to an LLM and ask whether it
looks important.

Instead:

```text
sensor reading
      ↓
Rust analysis
      ↓
thresholds
      ↓
cooldowns + hysteresis
      ↓
EnvironmentEvent
      ↓
LLM commentary
```

This keeps environmental behavior:

- predictable;
- testable;
- reproducible;
- inexpensive;
- independent of model personality.

It also lets the agent have personality without giving that personality
control over environmental thresholds.

**The language model can be sarcastic. The threshold logic cannot.**

---

## Local-first

The primary runtime can operate locally:

```text
Arduino       local
telemetry     local
history       local
monitoring    local
MCP           local
Ollama        local
Kokoros       local
audio         local
```

This makes Kūchō useful as an experiment in connecting local AI agents to
physical environments without requiring the core sensor loop to depend on a
remote AI service.

---

## Configuration

Runtime configuration is externalized rather than embedding developer-machine
paths in the application.

See:

```text
.env.example
```

for the supported configuration variables.

These include configuration for:

- serial hardware;
- weather-server networking;
- Ollama;
- chat and commentary models;
- MCP;
- speech;
- Kokoros;
- voice and speech speed;
- platform-specific ONNX Runtime configuration.

---

## Tests

Run the workspace checks:

```bash
just check
```

The environmental monitor has focused tests covering behavior such as:

- initialization without false events;
- temperature-change detection;
- event cooldowns;
- independent temperature/humidity cooldowns;
- safety-state recovery;
- hysteresis near thresholds.

For example:

```bash
cargo test -p weather-actix-server monitor
```

The server also contains integration coverage for its HTTP/MCP/event
boundaries.

---

## Documentation

| Document                                             | Purpose                                                |
| ---------------------------------------------------- | ------------------------------------------------------ |
| [`docs/setup-ubuntu.md`](docs/setup-ubuntu.md)       | Ubuntu installation and hardware setup                 |
| [`docs/setup-macos.md`](docs/setup-macos.md)         | macOS installation, including Intel ONNX Runtime notes |
| [`docs/architecture.md`](docs/architecture.md)       | System architecture and design decisions               |
| [`docs/troubleshooting.md`](docs/troubleshooting.md) | Common failures and diagnostic workflow                |
| [`docs/hardware.md`](docs/hardware.md)               | Arduino/DHT11 wiring and firmware hardware setup       |

If something fails unexpectedly, start with:

```bash
just doctor
```

then see the troubleshooting guide.

---

## Current limitations

Kūchō is still an experimental system.

Notable current limitations include:

- autonomous output can interrupt the interactive terminal prompt;
- multiple closely timed announcements can produce overlapping speech;
- speech setup remains platform-dependent;
- Intel macOS requires additional ONNX Runtime configuration;
- environmental thresholds are currently designed for the project's existing
  sensor setup rather than a generalized hardware abstraction.

These are deliberately visible rather than hidden behind the LLM layer.

---

## Project philosophy

Kūchō started as a weather-station project but evolved into an experiment in
giving a local AI agent a small amount of physical awareness.

The architecture keeps three concerns separate:

```text
SENSE
physical hardware tells us what is happening

DECIDE
deterministic software decides whether it matters

EXPRESS
the local AI decides how to communicate it
```

That separation is the core of the project.

The sensors provide the facts.

Rust provides the judgment.

Kūchō provides the attitude.

### Hardware

Build and wire the sensor node first:

[`docs/hardware.md`](docs/hardware.md)
