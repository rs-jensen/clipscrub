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

Useful for piping or quick one-off cleaning

---


