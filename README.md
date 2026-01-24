# Clipboard Scrubber

Clipboard Scrubber is a small CLI tool that runs in the background and cleans URLs when you copy them. It removes tracking parameters such as UTM tags, ref IDs, and affiliate markers, so you always paste clean, privacy-friendly links.

## What it does
- Monitors your clipboard for URLs
- Strips common tracking and analytics parameters
- Applies domain-specific cleaning rules (YouTube, Twitter/X, Amazon, etc.)
- Supports custom rules via a config file
- Optional TUI to see what was cleaned and basic stats

## Why it exists
Tracking parameters are everywhere and leak data when you share links. Clipboard Scrubber removes them automatically, without browser extensions or manual cleanup.

## How it works
When a URL is copied, it is parsed, cleaned based on global and domain rules, and written back to the clipboard. Non-URL clipboard content is ignored.

## Configuration
A `config.toml` file is created on first run. You can:
- Add or remove parameters to strip
- Define per-domain rules
- Whitelist domains
- Enable aggressive cleaning

## Use cases
- Sharing links without trackers
- Improving privacy by default
- Keeping URLs short and readable

## Status
Early-stage but functional. Focused on correctness, transparency, and minimalism.
