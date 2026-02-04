use std::{
    borrow::Cow,
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
    thread,
    time::{Duration, Instant},
};

use arboard::Clipboard;
use once_cell::sync::Lazy;
use regex::Regex;
use url::Url;

use crate::{Config, Stats, CleanEvent};

static REGEX_CACHE: Lazy<RwLock<HashMap<String, Regex>>> = Lazy::new(|| RwLock::new(HashMap::new()));

pub struct ClipboardWorker {
    events: Arc<Mutex<Vec<CleanEvent>>>,
    stats: Arc<Mutex<Stats>>,
    config: Arc<Config>,
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
            config,
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

            if let Some((cleaned, removed, domain)) = self.scrub_url(&txt) {
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

    pub fn scrub_url(&self, input: &str) -> Option<(String, Vec<String>, String)> {
        let trimmed = input.trim();
        if !trimmed.starts_with("http") {
            return None;
        }
    
        let mut parsed = Url::parse(trimmed).ok()?;
        let domain = self.extract_domain(&parsed);
        
        if self.config.whitelist_domains.iter().any(|d| domain.contains(d)) {
            return None;
        }
        
        let mut removed = Vec::new();
        let mut modified = false;
        let domain_rule = self.config.domain_rules.get(&domain);
        
        if let Some(rule) = domain_rule {
            if let Some(ref patterns) = rule.strip_path_patterns {
                let path = parsed.path();
                let mut new_path = Cow::Borrowed(path);
                
                for pattern in patterns {
                    if self.matches_regex(&new_path, pattern) {
                        if let Ok(re) = Regex::new(pattern) {
                             new_path = Cow::Owned(re.replace_all(&new_path, "").to_string());
                             modified = true;
                        }
                    }
                }
                if let Cow::Owned(p) = new_path {
                    parsed.set_path(&p);
                }
            }
        }
        
        let current_query: Vec<(String, String)> = parsed.query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
    
        if !current_query.is_empty() {
            let mut new_query = Vec::with_capacity(current_query.len());
            let mut query_changed = false;
    
            for (k, v) in current_query {
                let dominated = k.to_lowercase();
                let mut should_remove = false;
                let mut is_whitelisted = false;
    
                if let Some(rule) = domain_rule {
                    if let Some(ref keep) = rule.keep_only {
                        if !keep.iter().any(|allowed| allowed.to_lowercase() == dominated) {
                            should_remove = true;
                        } else {
                            is_whitelisted = true;
                        }
                    } else if rule.params.iter().any(|p| p.to_lowercase() == dominated) {
                        should_remove = true;
                    }
                }
    
                if !should_remove && !is_whitelisted {
                    if self.config.global_params.iter().any(|p| p.to_lowercase() == dominated) {
                        should_remove = true;
                    } else if self.matches_custom_pattern(&dominated, &self.config.custom_patterns) {
                        should_remove = true;
                    } else if self.config.aggressive_mode {
                        let suspicious = ["track", "click", "ref", "campaign", "source", "aff", "partner"];
                        if suspicious.iter().any(|s| dominated.contains(s)) {
                            should_remove = true;
                        }
                    }
                }
    
                if should_remove {
                    removed.push(k);
                    query_changed = true;
                } else {
                    new_query.push((k, v));
                }
            }
    
            if query_changed {
                modified = true;
                if new_query.is_empty() {
                    parsed.set_query(None);
                } else {
                    let mut qp = parsed.query_pairs_mut();
                    qp.clear();
                    for (k, v) in new_query {
                        qp.append_pair(&k, &v);
                    }
                }
            }
        }
        
        if self.config.strip_fragments && parsed.fragment().is_some() {
            parsed.set_fragment(None);
            modified = true;
        }
        
        if !modified {
            return None;
        }
        
        let mut result = parsed.to_string();
        if result.ends_with('?') { result.pop(); }
        
        Some((result, removed, domain))
    }

    fn extract_domain(&self, url: &Url) -> String {
        url.host_str()
            .map(|h| {
                let parts: Vec<&str> = h.split('.').collect();
                if parts.len() >= 2 {
                    format!("{}.{}", parts[parts.len()-2], parts[parts.len()-1])
                } else {
                    h.to_string()
                }
            })
            .unwrap_or_default()
    }

    fn matches_regex(&self, text: &str, pattern: &str) -> bool {
        {
            let cache = REGEX_CACHE.read().unwrap();
            if let Some(re) = cache.get(pattern) {
                return re.is_match(text);
            }
        }
        
        if let Ok(re) = Regex::new(pattern) {
            let hit = re.is_match(text);
            let mut cache = REGEX_CACHE.write().unwrap();
            cache.insert(pattern.to_string(), re);
            hit
        } else {
            false
        }
    }

    fn matches_custom_pattern(&self, param: &str, patterns: &[String]) -> bool {
        {
            let cache = REGEX_CACHE.read().unwrap();
            for pattern in patterns {
                if let Some(re) = cache.get(pattern) {
                    if re.is_match(param) {
                        return true;
                    }
                }
            }
        }
    
        let mut cache = REGEX_CACHE.write().unwrap();
        for pattern in patterns {
            if cache.contains_key(pattern) {
                continue; 
            }
            if let Ok(re) = Regex::new(pattern) {
                let hit = re.is_match(param);
                cache.insert(pattern.clone(), re);
                if hit {
                    return true;
                }
            }
        }
    
        false
    }
}
