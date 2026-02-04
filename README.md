---

# ClipScrub

ClipScrub is a small, fast, privacy-focused clipboard cleaner written in Rust.

It runs in the background and watches your clipboard for URLs. When it detects one, it automatically removes tracking parameters, affiliate tags, and other unnecessary junk before you paste the link anywhere else.

The goal is simple: clean links with zero friction and zero data leaving your machine.

The project includes a terminal UI (TUI) for live monitoring, safe config handling, and low-latency processing using cached regex rules.

The release binary is located at:

```
./target/release/clipscrub
```

---

## Usage

### Interactive mode (TUI)

Start ClipScrub normally to launch the monitoring interface:

```bash
./clipscrub
```

Key bindings:

* Space / p — Pause or resume monitoring
* Tab — Switch tabs (Live Feed / Stats / Top Domains)
* j / k — Scroll through history
* c — Clear event history
* q — Quit

---

### Daemon mode

Run ClipScrub silently in the background without the UI:

```bash
./clipscrub --daemon
```

---

### Single URL mode

Clean a single URL directly from the command line:

```bash
./clipscrub --clean "https://example.com?utm_source=test"
```

Useful for piping or quick one-off cleaning.

---

## Configuration

A `config.toml` file is generated automatically on first run.

Default locations:

* Linux
  `~/.config/clipscrub/config.toml`

* macOS
  `~/Library/Application Support/com.clipscrub.clipscrub/config.toml`

* Windows
  `%APPDATA%\clipscrub\clipscrub\config.toml`

---

### Environment variables

Some behavior can be overridden without touching the config file:

* `CLIPSCRUB_CONFIG`
  Path to a custom config file

* `CLIPSCRUB_AGGRESSIVE=1`
  Enables more aggressive heuristics (removes params containing `id`, `ref`, `track`, etc.)

* `CLIPSCRUB_STRIP_FRAGMENTS=1`
  Removes URL fragments (`#...`)

---

## Architecture notes

ClipScrub is intentionally simple and focused:

* ClipboardWorker
  Runs in a separate thread and polls the system clipboard. Handles parsing and cleaning logic.

* UrlCleaner
  Owns the regex cache and domain-specific rules.

* TUI engine
  Built with ratatui, using double buffering to avoid flicker.

* State sharing
  Uses `Arc<Mutex<State>>` to safely share data between the worker and UI threads.

---

## Why this exists

Most links today are bloated with tracking and analytics parameters. ClipScrub removes that noise automatically, locally, and without breaking your flow.

No telemetry.
No network calls.
No cloud.

Just clean links.

---
