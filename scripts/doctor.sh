#!/usr/bin/env bash

set -u

PASS=0
WARN=0
FAIL=0

green='\033[0;32m'
yellow='\033[0;33m'
red='\033[0;31m'
dim='\033[2m'
reset='\033[0m'

ok() {
	PASS=$((PASS + 1))
	printf "${green}[✓]${reset} %s\n" "$1"
}

warn() {
	WARN=$((WARN + 1))
	printf "${yellow}[!]${reset} %s\n" "$1"
}

fail() {
	FAIL=$((FAIL + 1))
	printf "${red}[✗]${reset} %s\n" "$1"
}

section() {
	local title="$1"

	printf '\n%s\n' "$title"
	printf '%*s\n' "${#title}" '' | tr ' ' '-'
}

has_command() {
	command -v "$1" >/dev/null 2>&1
}

check_file() {
	local label="$1"
	local path="$2"

	if [[ -n "$path" && -f "$path" ]]; then
		ok "$label: $path"
	else
		fail "$label"

		if [[ -n "$path" ]]; then
			printf "    ${dim}Expected: %s${reset}\n" "$path"
		fi
	fi
}

check_executable() {
	local label="$1"
	local path="$2"

	if [[ -n "$path" && -x "$path" ]]; then
		ok "$label: $path"
	else
		fail "$label"

		if [[ -n "$path" ]]; then
			printf "    ${dim}Expected executable: %s${reset}\n" "$path"
		fi
	fi
}

printf '\nKūchō setup doctor\n'
printf '%s\n' '=================='

section "System"

if has_command cargo; then
	ok "Cargo found: $(cargo --version)"
else
	fail "Cargo not found"
	printf '    Install Rust from https://rustup.rs\n'
fi

if has_command rustc; then
	ok "Rust compiler found: $(rustc --version)"
else
	fail "rustc not found"
fi

if has_command git; then
	ok "Git found"
else
	fail "Git not found"
fi

if has_command just; then
	ok "just found"
else
	warn "just not found"
	printf '    Install it to use the documented shortcut commands.\n'
fi

section "Firmware toolchain"

if has_command rustup && rustup toolchain list 2>/dev/null | grep -q '^nightly'; then
	ok "Rust nightly installed"
else
	warn "Rust nightly not installed"
	printf '    Fix: rustup toolchain install nightly --component rust-src\n'
fi

if has_command avr-gcc; then
	ok "avr-gcc found"
else
	warn "avr-gcc not found"
	printf '    Required only when building/flashing AVR firmware.\n'
fi

if has_command avrdude; then
	ok "avrdude found"
else
	warn "avrdude not found"
	printf '    Ubuntu fix: sudo apt install avrdude\n'
fi

if has_command ravedude; then
	ok "ravedude found"
else
	warn "ravedude not found"
	printf '    Required only when flashing the Arduino.\n'
	printf '    Fix: cargo install ravedude\n'
fi

section "Native build dependencies"

if has_command pkg-config; then
	ok "pkg-config found"
else
	fail "pkg-config not found"
	printf '    Ubuntu fix: sudo apt install pkg-config\n'
fi

if has_command cmake; then
	ok "cmake found"
else
	warn "cmake not found"
	printf '    Required when building Kokoros from source.\n'
	printf '    Ubuntu fix: sudo apt install cmake\n'
fi

if has_command clang; then
	ok "clang found"
else
	warn "clang not found"
	printf '    Required when building Kokoros/espeak-rs-sys from source.\n'
	printf '    Ubuntu fix: sudo apt install clang libclang-dev\n'
fi

section "Ollama"

OLLAMA_URL="${KUCHO_OLLAMA_URL:-http://127.0.0.1:11434}"
CHAT_MODEL="${KUCHO_CHAT_MODEL:-qwen3:4b}"
COMMENTARY_MODEL="${KUCHO_COMMENTARY_MODEL:-qwen2.5:3b}"

if has_command ollama; then
	ok "Ollama binary found"
else
	fail "Ollama binary not found"
fi

if curl -fsS "${OLLAMA_URL}/api/tags" >/dev/null 2>&1; then
	ok "Ollama reachable at ${OLLAMA_URL}"

	tags="$(curl -fsS "${OLLAMA_URL}/api/tags" 2>/dev/null || true)"

	if printf '%s' "$tags" | grep -q "\"name\":\"${CHAT_MODEL}\""; then
		ok "Chat model installed: ${CHAT_MODEL}"
	else
		fail "Chat model missing: ${CHAT_MODEL}"
		printf '    Fix: ollama pull %s\n' "$CHAT_MODEL"
	fi

	if printf '%s' "$tags" | grep -q "\"name\":\"${COMMENTARY_MODEL}\""; then
		ok "Commentary model installed: ${COMMENTARY_MODEL}"
	else
		fail "Commentary model missing: ${COMMENTARY_MODEL}"
		printf '    Fix: ollama pull %s\n' "$COMMENTARY_MODEL"
	fi
else
	fail "Ollama is not reachable at ${OLLAMA_URL}"
fi

section "Weather server"

SERIAL_PORT="${SERIAL_PORT:-/dev/ttyACM0}"
SERVER_URL="${KUCHO_MCP_URL:-http://127.0.0.1:8080/mcp}"
SERVER_BASE="${SERVER_URL%/mcp}"

if [[ -e "$SERIAL_PORT" ]]; then
	ok "Serial device exists: ${SERIAL_PORT}"

	if [[ -r "$SERIAL_PORT" && -w "$SERIAL_PORT" ]]; then
		ok "Serial device is readable and writable"
	else
		warn "Serial device exists but current user may not have access"
		printf '    Check: ls -l "%s"\n' "$SERIAL_PORT"

		if [[ "$(uname -s)" == "Linux" ]]; then
			printf '    Ubuntu fix: sudo usermod -aG dialout "$USER"\n'
			printf '    Then log out and back in.\n'
		fi
	fi
else
	warn "Serial device not found: ${SERIAL_PORT}"
	printf '    Set SERIAL_PORT to your Arduino device.\n'
fi

if curl -fsS "${SERVER_BASE}/health" >/dev/null 2>&1; then
	ok "Weather server reachable: ${SERVER_BASE}"

	if curl -fsS "${SERVER_BASE}/api/v1/telemetry" >/dev/null 2>&1; then
		ok "Live telemetry available"
	else
		warn "Server is running, but live telemetry is unavailable"
		printf '    Check the Arduino, serial port, and baud rate.\n'
	fi
else
	warn "Weather server is not currently running"
	printf '    Start it with: just server\n'
fi

section "Speech"

SPEECH_ENABLED="${KUCHO_SPEECH:-false}"

if [[ "$SPEECH_ENABLED" == "true" ]]; then
	check_executable \
		"Kokoros binary" \
		"${KUCHO_KOKO_BINARY:-}"

	check_file \
		"Kokoro model" \
		"${KUCHO_KOKORO_MODEL:-}"

	check_file \
		"Kokoro voices" \
		"${KUCHO_KOKORO_VOICES:-}"

	if [[ "$(uname -s)" == "Darwin" ]]; then
		if has_command afplay; then
			ok "Audio player found: afplay"
		else
			fail "afplay not found"
		fi
	elif [[ "$(uname -s)" == "Linux" ]]; then
		if has_command aplay; then
			ok "Audio player found: aplay"
		else
			fail "aplay not found"
			printf '    Ubuntu fix: sudo apt install alsa-utils\n'
		fi
	else
		warn "Automatic audio playback is not supported on this OS"
	fi

	if [[ -n "${ORT_DYLIB_PATH:-}" ]]; then
		check_file \
			"ONNX Runtime dynamic library" \
			"$ORT_DYLIB_PATH"
	else
		ok "ORT_DYLIB_PATH not set (normal unless Kokoros uses dynamic loading)"
	fi
else
	ok "Speech disabled"
fi

section "Summary"

printf "${green}%d passed${reset}, ${yellow}%d warnings${reset}, ${red}%d failed${reset}\n" \
	"$PASS" "$WARN" "$FAIL"

if [[ "$FAIL" -gt 0 ]]; then
	exit 1
fi
