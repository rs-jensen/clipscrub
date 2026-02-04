use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DomainRule {
    pub params: Vec<String>,
    pub keep_only: Option<Vec<String>>,
    pub strip_path_patterns: Option<Vec<String>>,
}

impl DomainRule {
    pub fn new() -> Self {
        Self {
            params: Vec::new(),
            keep_only: None,
            strip_path_patterns: None,
        }
    }

    pub fn block(mut self, params: &[&str]) -> Self {
        self.params.extend(params.iter().map(|&s| s.to_string()));
        self
    }

    pub fn allow_only(mut self, params: &[&str]) -> Self {
        self.keep_only = Some(params.iter().map(|&s| s.to_string()).collect());
        self
    }

    pub fn strip_path(mut self, patterns: &[&str]) -> Self {
        let mut current = self.strip_path_patterns.unwrap_or_default();
        current.extend(patterns.iter().map(|&s| s.to_string()));
        self.strip_path_patterns = Some(current);
        self
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub global_params: Vec<String>,
    pub domain_rules: HashMap<String, DomainRule>,
    pub whitelist_domains: Vec<String>,
    pub custom_patterns: Vec<String>,
    pub strip_fragments: bool,
    pub normalize_urls: bool,
    pub aggressive_mode: bool,
}

impl Config {
    pub fn load() -> Self {
        let path = Self::get_path();
        
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = toml::from_str(&content) {
                    return config;
                }
            }
        }
        
        let config = Self::default();
        config.save();
        config
    }

    pub fn save(&self) {
        let path = Self::get_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(content) = toml::to_string_pretty(self) {
            let _ = fs::write(path, content);
        }
    }

    pub fn get_path() -> PathBuf {
        if let Some(proj_dirs) = ProjectDirs::from("com", "clipscrub", "clipscrub") {
            proj_dirs.config_dir().join("config.toml")
        } else {
            PathBuf::from("clipscrub.toml")
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut rules = HashMap::new();

        rules.insert("youtube.com".into(), DomainRule::new()
            .block(&["si", "feature", "pp", "embeds_referring_euri", "source_ve_path"])
            .allow_only(&["v", "t", "list", "index"])
        );
        
        rules.insert("twitter.com".into(), DomainRule::new()
            .block(&["s", "t", "ref_src", "ref_url"])
        );
        
        rules.insert("x.com".into(), DomainRule::new()
            .block(&["s", "t", "ref_src", "ref_url"])
        );
        
        rules.insert("amazon.com".into(), DomainRule::new()
            .block(&[
                "tag", "linkCode", "linkId", "ref", "ref_", 
                "pf_rd_r", "pf_rd_p", "pf_rd_s", "pf_rd_t", "pf_rd_i", 
                "pd_rd_r", "pd_rd_w", "pd_rd_wg", "psc", "content-id", 
                "crid", "sprefix", "th"
            ])
            .strip_path(&[r"/ref=[^/\?]*"])
        );
        
        rules.insert("facebook.com".into(), DomainRule::new()
            .block(&["fbclid", "__tn__", "__cft__", "ref", "fref", "hc_ref"])
        );
        
        rules.insert("instagram.com".into(), DomainRule::new()
            .block(&["igshid", "igsh", "utm_source"])
        );
        
        rules.insert("tiktok.com".into(), DomainRule::new()
            .block(&["_t", "_r", "is_from_webapp", "sender_device", "enter_method"])
        );
        
        rules.insert("linkedin.com".into(), DomainRule::new()
            .block(&["trk", "trkInfo", "originalReferer", "upsellOrderOrigin", "midToken", "midSig", "lipi"])
        );
        
        rules.insert("reddit.com".into(), DomainRule::new()
            .block(&["share_id", "utm_medium", "utm_source", "utm_name", "context", "ref", "ref_source"])
        );
        
        rules.insert("spotify.com".into(), DomainRule::new()
            .block(&["si", "context", "nd"])
        );

        rules.insert("ebay.com".into(), DomainRule::new()
            .block(&[
                "_trkparms", "_trksid", "amdata", "mkevt", "mkcid", "mkrid", 
                "campid", "toolid", "customid"
            ])
        );

        rules.insert("aliexpress.com".into(), DomainRule::new()
            .block(&[
                "spm", "algo_pvid", "algo_expid", "btsid", "ws_ab_test", 
                "pdp_npi", "scm", "scm_id", "scm-url", "pvid", "utparam", 
                "gatewayAda498", "_t", "sk", "aff_fcid", "aff_fsk", 
                "aff_platform", "aff_trace_key", "terminal_id"
            ])
        );

        let global_params = vec![
            "utm_source", "utm_medium", "utm_campaign", "utm_term", "utm_content", 
            "utm_id", "utm_source_platform", "utm_creative_format", "utm_marketing_tactic",
            "fbclid", "gclid", "gclsrc", "dclid", "gbraid", "wbraid", "msclkid",
            "mc_eid", "mc_cid",
            "oly_anon_id", "oly_enc_id",
            "_openstat", "vero_id", "vero_conv",
            "affiliate", "aff", "partner", "ref",
            "click", "clickid", "click_id",
            "hsCtaTracking", "hsa_acc", "hsa_cam", "hsa_grp", "hsa_ad", "hsa_src", 
            "hsa_tgt", "hsa_kw", "hsa_mt", "hsa_net", "hsa_ver",
            "mkt_tok", "trk", "trkid",
            "s_kwcid", "ef_id",
            "__s", "_branch_match_id", "_branch_referrer",
            "irclickid", "irgwc",
            "_ga", "_gl", "_hsenc", "_hsmi",
            "yclid", "ymclid",
            "wickedid", "wickedsource",
            "rb_clickid", "sscid",
            "guccounter", "guce_referrer", "guce_referrer_sig",
            "__cf_chl_rt_tk",
        ].into_iter().map(String::from).collect();

        let custom_patterns = vec![
            r"^_[a-z]{2,4}$",
            r"^(ttclid|twclkid|li_fat_id)$",
        ].into_iter().map(String::from).collect();

        Self {
            global_params,
            domain_rules: rules,
            whitelist_domains: Vec::new(),
            custom_patterns,
            strip_fragments: false,
            normalize_urls: true,
            aggressive_mode: false,
        }
    }
}
