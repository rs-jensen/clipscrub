# Clipboard Scrubber

Clipboard Scrubber is a small CLI tool that runs in the background and cleans URLs when you copy them. It removes tracking parameters such as UTM tags, ref IDs, and affiliate markers, so you always paste clean, privacy-friendly links.

## Features

- **Zero-Latency Processing**: Uses advanced regex caching and `Cow` (Clone-on-Write) semantics for minimal memory footprint.
- **Privacy First**: Removes over 50+ known tracking parameters (`fbclid`, `gclid`, `utm_*`, `si`, etc.).
- **Domain Specific Rules**: Specialized handling for YouTube, Amazon, Twitter/X, Spotify, and more.
- **Path Stripping**: Automatically removes path-based tracking (common on Amazon links).
- **TUI Interface**: A beautiful terminal UI to see what's being cleaned in real-time.
- **Daemon Mode**: Run silently in the background without the UI.

## Installation

Building from source requires the Rust toolchain.

```bash
git clone https://github.com/rs-jensen/clipscrub.git
cd clipscrub
cargo build --release

## Use cases
- Sharing links without trackers
- Improving privacy by default
- Keeping URLs short and readable

## Status
Early-stage but functional.
