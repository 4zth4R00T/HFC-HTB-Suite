use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};
use zeroize::Zeroizing;

const CACHE_TTL: Duration = Duration::from_secs(20);
const MIN_GAP: Duration = Duration::from_millis(140);

pub const BASE_V4: &str = "https://labs.hackthebox.com/api/v4";
pub const BASE_V5: &str = "https://labs.hackthebox.com/api/v5";
pub const STORE_URL: &str = "https://www.hackthebox.com/hacker/subscriptions";
pub const TOKEN_URL: &str = "https://app.hackthebox.com/profile/settings";

pub struct ApiResponse {
    pub ok: bool,
    pub status: u16,
    pub data: Value,
}

impl ApiResponse {
    pub fn str_at(&self, pointer: &str) -> String {
        self.data
            .pointer(pointer)
            .and_then(|v| {
                if v.is_string() {
                    v.as_str().map(|s| s.to_string())
                } else if v.is_null() {
                    None
                } else {
                    Some(v.to_string())
                }
            })
            .unwrap_or_else(|| "-".to_string())
    }

    pub fn has(&self, pointer: &str) -> bool {
        self.str_at(pointer) != "-"
    }

    pub fn message(&self) -> String {
        let m = self.str_at("/message");
        if m == "-" {
            format!("HTTP {}", self.status)
        } else {
            m
        }
    }

    pub fn array_at(&self, pointer: &str) -> Vec<Value> {
        self.data
            .pointer(pointer)
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
    }

    pub fn first_array(&self, pointers: &[&str]) -> Vec<Value> {
        for p in pointers {
            let v = self.array_at(p);
            if !v.is_empty() {
                return v;
            }
        }
        if self.data.is_array() {
            return self.data.as_array().cloned().unwrap_or_default();
        }
        Vec::new()
    }
}

pub struct Client {
    http: reqwest::blocking::Client,
    token: std::cell::RefCell<Zeroizing<String>>,
    cache: std::cell::RefCell<HashMap<String, (Instant, u16, Value)>>,
    last_call: std::cell::RefCell<Option<Instant>>,
}

impl Client {
    pub fn new(token: &str) -> Self {
        let http = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(25))
            .https_only(true)
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());
        Client {
            http,
            token: std::cell::RefCell::new(Zeroizing::new(token.to_string())),
            cache: std::cell::RefCell::new(HashMap::new()),
            last_call: std::cell::RefCell::new(None),
        }
    }

    pub fn forget_cache(&self) {
        self.cache.borrow_mut().clear();
    }

    fn cached_get(&self, url: &str) -> ApiResponse {
        if let Some((stamp, status, data)) = self.cache.borrow().get(url) {
            if stamp.elapsed() < CACHE_TTL {
                return ApiResponse {
                    ok: (200..300).contains(status),
                    status: *status,
                    data: data.clone(),
                };
            }
        }
        let response = self.send(reqwest::Method::GET, url, None);
        if response.ok {
            self.cache.borrow_mut().insert(
                url.to_string(),
                (Instant::now(), response.status, response.data.clone()),
            );
        }
        response
    }

    pub fn token_copy(&self) -> String {
        self.token.borrow().to_string()
    }

    pub fn set_token(&self, token: &str) {
        *self.token.borrow_mut() = Zeroizing::new(token.to_string());
        self.forget_cache();
    }

    fn bearer(&self) -> Zeroizing<String> {
        Zeroizing::new(format!("Bearer {}", self.token.borrow().as_str()))
    }

    fn pace(&self) {
        let wait = {
            let last = self.last_call.borrow();
            match *last {
                Some(stamp) if stamp.elapsed() < MIN_GAP => Some(MIN_GAP - stamp.elapsed()),
                _ => None,
            }
        };
        if let Some(delay) = wait {
            std::thread::sleep(delay);
        }
        *self.last_call.borrow_mut() = Some(Instant::now());
    }

    fn auth_header(&self) -> reqwest::header::HeaderValue {
        let mut value = reqwest::header::HeaderValue::from_str(&self.bearer())
            .unwrap_or_else(|_| reqwest::header::HeaderValue::from_static("Bearer -"));
        value.set_sensitive(true);
        value
    }

    fn send(&self, method: reqwest::Method, url: &str, body: Option<Value>) -> ApiResponse {
        self.pace();
        let mut req = self
            .http
            .request(method, url)
            .header("Authorization", self.auth_header())
            .header("Accept", "application/json");
        if let Some(b) = &body {
            req = req.json(b);
        }
        match req.send() {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let ok = resp.status().is_success();
                let text = resp.text().unwrap_or_default();
                let data: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
                ApiResponse { ok, status, data }
            }
            Err(e) => ApiResponse {
                ok: false,
                status: 0,
                data: serde_json::json!({ "message": format!("fallo de red: {e}") }),
            },
        }
    }

    pub fn v4_get(&self, path: &str) -> ApiResponse {
        self.cached_get(&format!("{BASE_V4}{path}"))
    }

    pub fn v4_post(&self, path: &str, body: Value) -> ApiResponse {
        self.forget_cache();
        self.send(reqwest::Method::POST, &format!("{BASE_V4}{path}"), Some(body))
    }

    pub fn v5_get(&self, path: &str) -> ApiResponse {
        self.cached_get(&format!("{BASE_V5}{path}"))
    }

    pub fn v5_post(&self, path: &str, body: Value) -> ApiResponse {
        self.forget_cache();
        self.send(reqwest::Method::POST, &format!("{BASE_V5}{path}"), Some(body))
    }

    pub fn status_of(&self, path_base: &str, path: &str) -> (u16, u128) {
        let started = std::time::Instant::now();
        let res = self
            .http
            .get(format!("{path_base}{path}"))
            .header("Authorization", self.auth_header())
            .header("Accept", "application/json")
            .timeout(Duration::from_secs(12))
            .send();
        let ms = started.elapsed().as_millis();
        let code = res.map(|r| r.status().as_u16()).unwrap_or(0);
        (code, ms)
    }

    pub fn download(&self, url: &str, out_path: &Path) -> Result<u64, String> {
        let mut resp = self
            .http
            .get(url)
            .header("Authorization", self.auth_header())
            .timeout(Duration::from_secs(120))
            .send()
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status().as_u16()));
        }
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut file = opts.open(out_path).map_err(|e| e.to_string())?;
        std::io::copy(&mut resp, &mut file).map_err(|e| e.to_string())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Verdict {
    Accepted,
    Rejected,
    Unclear,
}

pub fn read_flag_answer(message: &str) -> Verdict {
    let m = message.to_lowercase();
    let rejected = [
        "incorrect",
        "not correct",
        "wrong",
        "invalid",
        "already",
        "failed",
        "no puede",
        "error",
    ];
    if rejected.iter().any(|k| m.contains(k)) {
        return Verdict::Rejected;
    }
    let accepted = [
        "correct",
        "owned",
        "congrat",
        "obtained",
        "accepted",
        "solved",
        "well done",
    ];
    if accepted.iter().any(|k| m.contains(k)) {
        return Verdict::Accepted;
    }
    Verdict::Unclear
}

pub fn read_action_answer(message: &str, ok: bool) -> Verdict {
    let m = message.to_lowercase();
    let refused = [
        "cannot",
        "can not",
        "can't",
        "could not",
        "couldn't",
        "not find",
        "no active",
        "please switch",
        "not allowed",
        "no puedes",
        "unauthorized",
        "forbidden",
        "already have",
        "error",
        "invalid",
        "failed",
        "requires",
        "upgrade",
        "vip",
    ];
    if refused.iter().any(|k| m.contains(k)) {
        return Verdict::Rejected;
    }
    let done = [
        "spawn",
        "started",
        "starting",
        "deployed",
        "playing",
        "being",
        "terminat",
        "reset",
        "extend",
        "success",
        "switched",
    ];
    if done.iter().any(|k| m.contains(k)) {
        return Verdict::Accepted;
    }
    if ok {
        Verdict::Accepted
    } else {
        Verdict::Rejected
    }
}

pub fn mentions_plan(message: &str) -> bool {
    let m = message.to_lowercase();
    [
        "vip",
        "upgrade",
        "subscription",
        "free server",
        "requires a",
        "not available on your plan",
        "premium",
    ]
    .iter()
    .any(|k| m.contains(k))
}

pub fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('_').to_string();
    if trimmed.is_empty() {
        "archivo".to_string()
    } else {
        trimmed
    }
}
