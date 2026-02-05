use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use arboard::Clipboard;
use crate::{Config, Stats, CleanEvent};
use crate::url_cleaner::UrlCleaner;

pub struct ClipboardWorker {
    events: Arc<Mutex<Vec<CleanEvent>>>,
    stats: Arc<Mutex<Stats>>,
    cleaner: UrlCleaner,
}

impl ClipboardWorker {
    pub fn new(
        events: Arc<Mutex<Vec<CleanEvent>>>,
        stats: Arc<Mutex<Stats>>,
        config: Arc<Config>,
    ) -> Self {
        Self {
            events,
            stats,
            cleaner: UrlCleaner::new(config),
        }
    }

    pub fn spawn(self, paused: Arc<Mutex<bool>>) {
        thread::spawn(move || {
            self.run(Some(paused));
        });
    }

    pub fn run(&self, paused: Option<Arc<Mutex<bool>>>) {
        let mut last_content = String::new();
        let mut error_backoff = 0;

        loop {
            thread::sleep(Duration::from_millis(250 + error_backoff));

            if let Some(ref p) = paused {
                if *p.lock().unwrap() {
                    continue;
                }
            }

            let mut clip = match Clipboard::new() {
                Ok(c) => {
                    error_backoff = 0;
                    c
                },
                Err(_) => {
                    error_backoff = std::cmp::min(error_backoff + 500, 5000);
                    continue;
                }
            };

            let txt = match clip.get_text() {
                Ok(t) => t,
                Err(_) => continue,
            };

            if txt == last_content {
                continue;
            }

            let trimmed = txt.trim();
            if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
                last_content = txt;
                continue;
            }

            // Her bruger vi nu UrlCleaner i stedet for intern logik
            if let Some((cleaned, removed, domain)) = self.cleaner.scrub(&txt) {
                if cleaned != txt {
                    if let Err(_) = clip.set_text(&cleaned) {
                        continue; 
                    }
                    
                    self.record_event(&txt, &cleaned, &domain, &removed);
                    last_content = cleaned;
                } else {
                    last_content = txt;
                }
            } else {
                last_content = txt;
            }
        }
    }

    // Proxy metode til one-shot cleaning fra main.rs
    pub fn scrub_url(&self, input: &str) -> Option<(String, Vec<String>, String)> {
        self.cleaner.scrub(input)
    }

    fn record_event(&self, original: &str, cleaned: &str, domain: &str, removed: &[String]) {
        let evt = CleanEvent {
            original: original.to_string(),
            cleaned: cleaned.to_string(),
            domain: domain.to_string(),
            removed_params: removed.to_vec(),
            timestamp: Instant::now(),
        };

        if let Ok(mut ev) = self.events.lock() {
            ev.insert(0, evt);
            if ev.len() > 100 {
                ev.truncate(100);
            }
        }

        if let Ok(mut st) = self.stats.lock() {
            st.total_cleaned += 1;
            st.params_removed += removed.len() as u64;
            st.bytes_saved += (original.len() - cleaned.len()) as u64;
            *st.domains.entry(domain.to_string()).or_insert(0) += 1;
        }
    }
}
