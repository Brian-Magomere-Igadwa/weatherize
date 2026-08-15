# Kūchō Architecture

Kūchō is a local-first ambient environmental assistant that combines physical
sensor telemetry, deterministic environmental monitoring, local LLM reasoning,
and optional local speech synthesis.

The system deliberately separates **measurement and safety logic** from
**language generation**.

The LLM does not decide whether an environmental change is significant.
Deterministic Rust code detects and filters those changes first. The agent is
responsible for interpreting user questions and turning already-detected events
into natural commentary.

## System overview

```text
                         ┌─────────────────┐
                         │     Arduino     │
                         │  DHT11 sensors  │
                         └────────┬────────┘
                                  │
                              USB serial
                                  │
                                  ▼
                 ┌────────────────────────────────┐
                 │     weather-actix-server       │
                 │                                │
                 │  Serial ingestion              │
                 │        │                       │
                 │        ▼                       │
                 │  Telemetry history             │
                 │        │                       │
                 │        ├── Trend analysis      │
                 │        │                       │
                 │        ▼                       │
                 │  Environment monitor           │
                 │   ├── detection                │
                 │   ├── hysteresis               │
                 │   └── cooldowns                │
                 │                                │
                 │  HTTP :8080                    │
                 │   ├── REST telemetry           │
                 │   ├── MCP tools                │
                 │   └── SSE event stream         │
                 └───────────┬───────────┬────────┘
                             │           │
                         MCP │           │ SSE
                             │           │
                             ▼           ▼
                    ┌──────────────────────────┐
                    │        cli-agent         │
                    │                          │
                    │ Interactive questions    │
                    │ Autonomous events        │
                    └───────┬─────────┬────────┘
                            │         │
                            │         │
                         Ollama    SpeechEngine
                            │         │
                            │         ▼
                            │      Kokoros
                            │         │
                            │         ▼
                            │   afplay / aplay
                            │         │
                            ▼         ▼
                         response   speakers
```

---

# Workspace structure

Kūchō is organized as a Rust workspace.

```text
weather-core/
firmware-avr/
weather-actix-server/
cli-agent/
```

Each crate owns a distinct layer of the system.

## `weather-core`

`weather-core` contains domain types and environmental rules that do not depend
on hardware, HTTP, Ollama, or the CLI.

This includes concepts such as:

- telemetry payloads;
- climate trends;
- metric direction;
- safety status;
- environmental events.

Keeping these types independent allows the server, firmware-facing code, and
tests to share the same domain vocabulary.

## `firmware-avr`

The firmware crate runs on the microcontroller.

Its responsibility is deliberately small:

```text
physical sensors
      ↓
measurement
      ↓
telemetry payload
      ↓
serial output
```

The firmware does not perform LLM reasoning or autonomous commentary.

## `weather-actix-server`

The Actix server is the central runtime boundary between physical telemetry and
higher-level consumers.

It owns:

- serial ingestion;
- current telemetry state;
- telemetry history;
- climate trend analysis;
- environmental event monitoring;
- MCP tools;
- REST endpoints;
- the SSE environmental-event stream.

## `cli-agent`

The agent is the human-facing runtime.

It owns:

- interactive CLI input;
- Ollama communication;
- MCP tool discovery and calls;
- environmental SSE consumption;
- autonomous commentary generation;
- optional speech synthesis and playback.

---

# Telemetry pipeline

The primary data path begins at the physical sensors.

```text
DHT11
  ↓
Arduino firmware
  ↓
serial JSON
  ↓
weather-actix-server
  ↓
TelemetrySample
  ├── current telemetry
  └── telemetry history
```

The server maintains both the most recent observation and a bounded history of
recent samples.

This allows Kūchō to answer two fundamentally different questions:

```text
"What is the temperature?"

          ↓

current telemetry
```

versus:

```text
"Has it been getting hotter?"

          ↓

telemetry history
          ↓
trend analysis
```

---

# Climate trend analysis

Trend analysis operates over a requested telemetry window.

A climate trend contains information such as:

```text
start value
current value
delta
rate per minute
direction
sample count
```

Temperature and humidity trends are evaluated independently.

For example:

```text
temperature
  start:      23.0°C
  current:    25.0°C
  delta:      +2.0°C
  direction:  RISING

humidity
  start:      50%
  current:    51%
  delta:      +1%
  direction:  RISING
```

The MCP layer exposes this analysis to the agent through the
`get_climate_trend` tool.

This means the LLM does not calculate trends from raw sensor samples itself.
The numerical analysis remains deterministic Rust code.

---

# Environmental event detection

Trend analysis alone does not imply that the user should be interrupted.

Kūchō therefore has a separate deterministic event-detection layer.

Significant changes currently include rapid temperature and humidity movement
and safety-status transitions.

Conceptually:

```text
ClimateTrend
     +
previous safety status
     +
current safety status
     ↓
detector
     ↓
Vec<EnvironmentEvent>
```

Examples include:

```text
TemperatureChangedRapidly
HumidityChangedRapidly
SafetyStatusChanged
```

The important architectural rule is:

> The LLM does not determine whether an environmental condition deserves an
> autonomous notification.

That decision belongs to deterministic monitoring code.

---

# Monitor state

A raw detector alone would generate noisy repeated events.

For example, if humidity remains above a threshold for several consecutive
samples, repeatedly announcing the same condition would make Kūchō unusable.

The environment monitor therefore maintains state across observations.

It provides two important controls.

## Cooldowns

Repeated rapid-change events are suppressed for a period after an event has
already been emitted.

Temperature and humidity have independent cooldown state.

This prevents behavior such as:

```text
Humidity jumped.
Humidity jumped.
Humidity jumped.
Humidity jumped.
```

from a single continuing environmental change.

## Hysteresis

Safety-state recovery uses hysteresis rather than switching state immediately
at the same boundary that triggered the condition.

Conceptually:

```text
enter STUFFY
    ↓
condition improves slightly
    ↓
remain STUFFY
    ↓
recovery threshold clearly crossed
    ↓
return to OPTIMAL
```

This prevents rapid oscillation around a threshold:

```text
OPTIMAL
STUFFY
OPTIMAL
STUFFY
OPTIMAL
```

Together, cooldowns and hysteresis make autonomous events stable enough for
human-facing notifications.

---

# Event distribution

Once the monitor emits an `EnvironmentEvent`, the weather server publishes it
through a Tokio broadcast channel.

```text
EnvironmentMonitor
       ↓
EnvironmentEvent
       ↓
broadcast::Sender
       ↓
/api/v1/events
       ↓
Server-Sent Events
```

The SSE endpoint keeps a long-lived HTTP connection open.

Each event is serialized and sent in SSE form:

```text
data: {...}

```

This gives the agent a push-based event channel.

The agent therefore does **not** need to poll the server repeatedly asking:

```text
Anything happen?
Anything happen?
Anything happen?
```

The server tells it when something has happened.

---

# MCP

MCP provides the request-driven side of Kūchō's agent integration.

The weather server currently exposes tools for querying environmental state.

## `get_indoor_climate`

Returns the latest available sensor reading.

Typical user questions:

```text
What's the temperature?
How humid is it in here?
What's the room like right now?
```

## `get_climate_trend`

Analyzes recent telemetry over a requested time window.

Typical user questions:

```text
Is it getting hotter?
Has humidity been rising?
How has the room changed over the last minute?
```

This produces an important separation:

```text
                   weather server

          ┌─────────────┴──────────────┐
          │                            │
     request-driven               event-driven
          │                            │
         MCP                          SSE
          │                            │
user asks something          environment changes
```

Both paths ultimately reach the same agent but serve different interaction
models.

---

# Agent architecture

The CLI agent operates two related flows.

## Interactive flow

```text
user input
    ↓
AgentEngine
    ↓
Ollama
    ↓
optional MCP tool call
    ↓
weather server
    ↓
tool result
    ↓
Ollama
    ↓
natural-language response
```

The interactive model can dynamically inspect the MCP tool definitions exposed
by the server.

This allows the weather server to remain the authority for physical data while
the LLM focuses on deciding which available tool is appropriate for the user's
question.

## Autonomous flow

The event flow begins without user input:

```text
environment changes
      ↓
deterministic monitor
      ↓
SSE
      ↓
cli-agent
      ↓
commentary model
      ↓
short natural observation
      ↓
terminal
      ↓
optional speech
```

The commentary model receives an event that has **already been approved for
notification** by deterministic logic.

Its responsibility is presentation, not detection.

---

# Dual-model design

Kūchō currently uses separate Ollama models for two workloads.

```text
qwen3:4b
   ↓
interactive conversation
MCP reasoning
tool calls

qwen2.5:3b
   ↓
short autonomous
environmental commentary
```

The interactive workload benefits from stronger reasoning and tool-use
behavior.

Autonomous commentary is smaller and more constrained, so a lighter model can
produce short observations without involving the full interactive reasoning
path.

Both models run locally through Ollama.

---

# Speech architecture

Speech is optional.

When enabled:

```text
EnvironmentEvent
      ↓
commentary model
      ↓
text
      ↓
SpeechEngine
      ↓
Kokoros
      ↓
WAV
      ↓
afplay (macOS)
or
aplay (Linux)
      ↓
speakers
```

The current default voice is:

```text
bm_lewis
```

with a slightly reduced speech rate.

Speech configuration is externalized through environment variables or CLI
configuration. No developer-machine paths are embedded in the application.

Kokoros may require an explicit ONNX Runtime dynamic library on some platforms,
particularly Intel macOS. This is a deployment concern of the speech layer and
does not affect the rest of the system.

See `setup-macos.md` for the Intel-specific workaround.

---

# Concurrency model

Several activities happen concurrently:

```text
serial ingestion
telemetry history updates
environment monitoring
HTTP requests
MCP calls
SSE connections
interactive CLI input
autonomous event consumption
speech generation/playback
```

Tokio provides the asynchronous runtime for the host-side services.

Different synchronization primitives are used for different state semantics:

```text
watch
  → latest-value telemetry state

RwLock
  → shared telemetry history

broadcast
  → fan-out environmental events
```

This distinction is intentional.

A current telemetry reading has "latest value" semantics, while environmental
events represent a stream that consumers need to observe as they occur.

---

# Why detection is deterministic

Kūchō intentionally avoids asking the LLM questions such as:

```text
"Does this temperature change seem important?"
```

The system instead uses:

```text
sensor values
    ↓
deterministic analysis
    ↓
deterministic thresholds
    ↓
monitor state
    ↓
event
    ↓
LLM wording
```

This has several advantages:

- predictable environmental behavior;
- testable thresholds;
- reproducible event decisions;
- lower LLM usage;
- fewer hallucination opportunities;
- clear separation between safety logic and personality.

The language model can be sarcastic.

The threshold logic cannot.

---

# Local-first design

Kūchō is designed so its primary runtime can remain local:

```text
sensors       local
server        local
history       local
detection     local
Ollama        local
Kokoros       local
audio         local
```

The core environmental loop therefore does not inherently require a remote AI
API.

This also keeps physical sensor data within the local runtime unless additional
integrations are deliberately introduced.

---

# Deployment model

Kūchō currently favors a host-native runtime for components that interact
directly with hardware or desktop facilities.

```text
Host native
├── weather-actix-server
├── cli-agent
├── Ollama
└── Kokoros

Hardware
└── Arduino firmware

Optional containers
└── observability infrastructure
```

Running the weather server and speech stack directly on the host avoids
unnecessary complexity around:

- USB serial passthrough;
- platform-specific serial device names;
- host audio access;
- local Ollama access;
- ONNX Runtime platform differences.

Docker remains useful for infrastructure that does not require those host
boundaries.

---

# Known v0.1 limitations

The initial release intentionally keeps several concerns simple.

## Speech queue

Speech is non-blocking from the agent's main interaction flow, but autonomous
announcements are not yet serialized through a dedicated speech queue.

Multiple events arriving close together can therefore produce overlapping
speech jobs.

A later release can introduce:

```text
event
  ↓
speech queue
  ↓
single speech worker
```

## Interactive terminal rendering

Autonomous events and interactive input share the terminal.

An environmental event can therefore appear while the user is typing.

A future CLI layer can provide asynchronous prompt-safe rendering.

## Speech portability

Kokoros is local and cross-platform in principle, but ONNX Runtime behavior
varies by platform.

Intel macOS currently requires additional dynamic-runtime configuration.

## Observability

The observability stack is optional and is not required for the primary Kūchō
runtime.

It should be treated separately from the core quick-start path.

---

# Design principles

Kūchō's architecture currently follows a few central principles:

1. **Physical measurements are authoritative.**
   The LLM never invents environmental readings.

2. **Numerical analysis is deterministic.**
   Rust computes trends and rates.

3. **Notification decisions are deterministic.**
   Thresholds, cooldowns, and hysteresis decide when an event exists.

4. **Language is probabilistic.**
   The LLM determines how an already-approved observation is communicated.

5. **Hardware and AI are loosely coupled.**
   MCP and SSE form explicit boundaries between the weather server and agent.

6. **Speech is optional.**
   Failure or absence of TTS must not prevent the core agent from operating.

7. **The primary runtime is local-first.**
   Sensor processing, AI inference, and speech can all execute on the user's
   machine.
