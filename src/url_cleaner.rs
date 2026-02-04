use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use once_cell::sync::Lazy;
use regex::Regex;
use url::Url;
use crate::{Config, DomainRule};

static REGEX_CACHE: Lazy<RwLock<HashMap<String, Regex>>> = Lazy::new(|| RwLock::new(HashMap::new()));

pub struct UrlCleaner {
    config: Arc<Config>,
}

impl UrlCleaner {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }

    pub fn scrub(&self, input: &str) -> Option<(String, Vec<String>, String)> {
        let trimmed = input.trim();
        if !trimmed.starts_with("http") {
            return None;
        }
    
        let mut parsed = match Url::parse(trimmed) {
            Ok(u) => u,
            Err(_) => return None,
        };

        let domain = self.extract_domain(&parsed);
        
        if self.config.whitelist_domains.iter().any(|d| domain.contains(d)) {
            return None;
        }
        
        let mut removed = Vec::new();
        let mut modified = false;
        let domain_rule = self.config.domain_rules.get(&domain);
        
        if let Some(rule) = domain_rule {
            if self.process_path_rules(&mut parsed, rule) {
                modified = true;
            }
        }
        
        let current_query: Vec<(String, String)> = parsed.query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
    
        if !current_query.is_empty() {
            let (new_query, query_removed) = self.filter_query_params(&current_query, &domain, domain_rule);
            
            if !query_removed.is_empty() {
                removed = query_removed;
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
        if result.ends_with('?') { 
            result.pop(); 
        }
        
        Some((result, removed, domain))
    }

    fn process_path_rules(&self, parsed: &mut Url, rule: &DomainRule) -> bool {
        let Some(ref patterns) = rule.strip_path_patterns else {
            return false;
        };

        let path = parsed.path();
        let mut new_path = Cow::Borrowed(path);
        let mut changed = false;
        
        for pattern in patterns {
            if self.matches_regex(&new_path, pattern) {
                if let Ok(re) = Regex::new(pattern) {
                        new_path = Cow::Owned(re.replace_all(&new_path, "").to_string());
                        changed = true;
                }
            }
        }
        
        if let Cow::Owned(p) = new_path {
            parsed.set_path(&p);
        }

        changed
    }

    fn filter_query_params(
        &self, 
        current_query: &[(String, String)], 
        _domain: &str, 
        rule: Option<&DomainRule>
    ) -> (Vec<(String, String)>, Vec<String>) {
        let mut new_query = Vec::with_capacity(current_query.len());
        let mut removed = Vec::new();

        for (k, v) in current_query {
            let dominated = k.to_lowercase();
            let mut should_remove = false;
            let mut is_whitelisted = false;

            if let Some(r) = rule {
                if let Some(ref keep) = r.keep_only {
                    if !keep.iter().any(|allowed| allowed.to_lowercase() == dominated) {
                        should_remove = true;
                    } else {
                        is_whitelisted = true;
                    }
                } else if r.params.iter().any(|p| p.to_lowercase() == dominated) {
                    should_remove = true;
                }
            }

            if !should_remove && !is_whitelisted {
                if self.config.global_params.iter().any(|p| p.to_lowercase() == dominated) {
                    should_remove = true;
                } else if self.matches_custom_pattern(&dominated) {
                    should_remove = true;
                } else if self.config.aggressive_mode {
                    let suspicious = ["track", "click", "ref", "campaign", "source", "aff", "partner"];
                    if suspicious.iter().any(|s| dominated.contains(s)) {
                        should_remove = true;
                    }
                }
            }

            if should_remove {
                removed.push(k.clone());
            } else {
                new_query.push((k.clone(), v.clone()));
            }
        }

        (new_query, removed)
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

    fn matches_custom_pattern(&self, param: &str) -> bool {
        {
            let cache = REGEX_CACHE.read().unwrap();
            for pattern in &self.config.custom_patterns {
                if let Some(re) = cache.get(pattern) {
                    if re.is_match(param) {
                        return true;
                    }
                }
            }
        }
    
        let mut cache = REGEX_CACHE.write().unwrap();
        for pattern in &self.config.custom_patterns {
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
