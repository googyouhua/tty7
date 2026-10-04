use crate::core::cli_agent::CLIAgent;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleStarter {
    pub label: String,
    pub prompt: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRole {
    pub slug: String,
    pub name: String,
    pub base: CLIAgent,
    pub description: String,
    pub launch: String,
    /// Model override, empty for none. Appended as `{flag} {model}` after
    /// the launch words (see [`role_launch_argv`]).
    pub model: String,
    pub instructions: String,
    pub starters: Vec<RoleStarter>,
}

pub fn slug_valid(slug: &str) -> bool {
    !slug.is_empty()
        && slug.len() <= 64
        && slug
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

pub fn slug_from_name(name: &str) -> String {
    let mut s: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    while s.contains("--") {
        s = s.replace("--", "-");
    }
    let mut s = s.trim_matches('-').to_string();
    // Slugs cap at [`slug_valid`]'s 64: cut, then drop a trailing dash the
    // cut may leave, so a minted slug always saves.
    if s.len() > 64 {
        s.truncate(64);
        s = s.trim_end_matches('-').to_string();
    }
    if s.is_empty() { "role".to_string() } else { s }
}

pub fn roles_dir() -> Option<PathBuf> {
    crate::core::config::config_dir_path().map(|d| d.join("roles"))
}

fn load_one(dir: &Path, slug: &str) -> Option<AgentRole> {
    let file = dir.join("role.json");
    let text = match std::fs::read_to_string(&file) {
        Ok(text) => text,
        Err(e) => {
            log::warn!(
                "ignoring role at {}: cannot read role.json: {e}",
                dir.display()
            );
            return None;
        }
    };
    let v: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            log::warn!(
                "ignoring role at {}: unparseable role.json: {e}",
                dir.display()
            );
            return None;
        }
    };
    let file_slug = v.get("slug").and_then(|s| s.as_str()).unwrap_or("");
    if file_slug != slug || !slug_valid(file_slug) {
        log::warn!(
            "ignoring role at {}: slug {file_slug:?} does not match directory {slug:?} or is invalid",
            dir.display()
        );
        return None;
    }
    let base_str = v.get("base").and_then(|s| s.as_str()).unwrap_or("");
    let Some(base) = CLIAgent::from_slug(base_str) else {
        log::warn!(
            "ignoring role at {}: unknown base agent slug {base_str:?}",
            dir.display()
        );
        return None;
    };
    let launch = v
        .get("launch")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    // Empty stays empty: it launches the base binary, like an unset
    // `agent_launch` entry does.
    let mut starters: Vec<RoleStarter> = v
        .get("starters")
        .and_then(|s| s.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|e| {
                    Some(RoleStarter {
                        label: e
                            .get("label")
                            .and_then(|l| l.as_str())
                            .unwrap_or("")
                            .trim()
                            .to_string(),
                        prompt: e
                            .get("prompt")
                            .and_then(|p| p.as_str())
                            .unwrap_or("")
                            .to_string(),
                    })
                })
                .filter(|s| !s.label.is_empty() && !s.prompt.trim().is_empty())
                .collect()
        })
        .unwrap_or_default();
    starters.truncate(3);
    Some(AgentRole {
        slug: file_slug.to_string(),
        name: v
            .get("name")
            .and_then(|s| s.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(file_slug)
            .to_string(),
        base,
        description: v
            .get("description")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string(),
        launch,
        model: v
            .get("model")
            .and_then(|s| s.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("")
            .to_string(),
        instructions: v
            .get("instructions")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string(),
        starters,
    })
}

pub fn load_roles(dir: &Path) -> Vec<AgentRole> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<AgentRole> = entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
        .filter_map(|slug| load_one(&dir.join(&slug), &slug))
        .collect();
    out.sort_by(|a, b| a.slug.cmp(&b.slug));
    out
}

pub fn role_launch_program(role: &AgentRole) -> Option<String> {
    // An empty launch line means the bare base binary.
    Some(
        crate::core::cli_agent::launch_program(&role.launch)
            .unwrap_or_else(|| role.base.binary().to_string()),
    )
}

/// The argv a role launches (and resumes) with: the launch words plus
/// `{flag} {model}` when the role names a model. The flag comes from the
/// same table as the model list ([`model_flag`]).
///
/// The launch line is the final escape hatch: when it already names the
/// flag (bare or `flag=value`), the dropdown value is not appended again.
pub fn role_launch_argv(role: &AgentRole) -> Vec<String> {
    let mut argv: Vec<String> = role.launch.split_whitespace().map(str::to_string).collect();
    if argv.is_empty() {
        argv.push(role.base.binary().to_string());
    }
    let model = role.model.trim();
    if !model.is_empty()
        && let Some(flag) = model_flag(role.base)
        && !argv
            .iter()
            .any(|t| t == flag || t.starts_with(&format!("{flag}=")))
    {
        argv.push(flag.to_string());
        argv.push(model.to_string());
    }
    argv
}

/// The CLI flag a base agent takes its model under. First `models.json`,
/// then the built-in table — so users can correct a flag without a rebuild.
pub fn model_flag(base: CLIAgent) -> Option<&'static str> {
    if let Some(flag) = models_file_flag(base) {
        return Some(flag);
    }
    Some(match base {
        CLIAgent::Claude => "--model",
        CLIAgent::Codex => "--model",
        CLIAgent::Gemini => "--model",
        CLIAgent::OpenCode | CLIAgent::OpenCode2 => "--model",
        CLIAgent::Cursor => "--model",
        CLIAgent::Copilot => "--model",
        CLIAgent::Qwen => "--model",
        CLIAgent::Kimi => "--model",
        CLIAgent::Droid => "--model",
        CLIAgent::Grok => "--model",
        CLIAgent::QoderCLI | CLIAgent::QoderCLICn => "--model",
        CLIAgent::CodeBuddy => "--model",
        CLIAgent::Crush => "--model",
        CLIAgent::Pi => "--model",
        CLIAgent::OhMyPi => "--model",
        CLIAgent::PrimeAgent => "--model",
        CLIAgent::TraeCode => "--model",
        CLIAgent::Goose => "--model",
        CLIAgent::Vibe => "--model",
        CLIAgent::Auggie => "--model",
        CLIAgent::Amp => "--model",
        CLIAgent::Aider => "--model",
        CLIAgent::Hermes => "--model",
        CLIAgent::Antigravity => "--model",
        CLIAgent::Empryo => "--model",
    })
}

/// Read the per-base `{flag, models[]}` table out of `models.json` beside
/// `roles/`. Missing file, bad JSON and unknown bases all read as empty —
/// the dropdown then hides and the launch line rules alone.
fn models_file() -> serde_json::Value {
    let Some(path) = crate::core::config::config_dir_path().map(|d| d.join("models.json")) else {
        return serde_json::json!({});
    };
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(serde_json::json!({}))
}

/// The `models.json` flag override for `base`, if it names one. Leaked on
/// first read like the keymap's role action names: the file is read through
/// this one choke point, so a later refresh only touches here.
fn models_file_flag(base: CLIAgent) -> Option<&'static str> {
    let flag = models_file()
        .get(base.slug())?
        .get("flag")?
        .as_str()?
        .trim()
        .to_string();
    (!flag.is_empty()).then(|| Box::leak(flag.into_boxed_str()) as &str)
}

/// The `models.json` model list for `base`, if it names any.
fn models_file_models(base: CLIAgent) -> Vec<String> {
    models_file()
        .get(base.slug())
        .and_then(|e| e.get("models"))
        .and_then(|m| m.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// A small built-in model table for the big four, used only when neither a
/// live fetch nor `models.json` names any. Stale the moment vendors move —
/// the file above is the correction path.
fn builtin_models(base: CLIAgent) -> &'static [&'static str] {
    match base {
        CLIAgent::Claude => &["opus", "sonnet", "haiku"],
        CLIAgent::Codex => &["gpt-5.1", "gpt-5-mini"],
        CLIAgent::Gemini => &["gemini-2.5-pro", "gemini-2.5-flash"],
        CLIAgent::OpenCode | CLIAgent::OpenCode2 => {
            &["opencode/gpt-5", "opencode/claude-sonnet-4-5"]
        }
        _ => &[],
    }
}

/// Models known without a live fetch: `models.json`, then the built-in
/// table. Used to seed the form immediately; the live fetch fills in
/// afterwards on a background task so a hung CLI never freezes the UI.
pub fn model_choices_fast(base: CLIAgent) -> Vec<String> {
    let out = models_file_models(base);
    if !out.is_empty() {
        return out;
    }
    builtin_models(base).iter().map(|s| s.to_string()).collect()
}

/// Models known right now for `base`: a live fetch first, then
/// `models.json`, then the built-in table. Empty means the caller falls
/// back to free text (or hides the picker).
pub fn model_choices(base: CLIAgent) -> Vec<String> {
    let mut out = fetch_models(base);
    if out.is_empty() {
        out = models_file_models(base);
    }
    if out.is_empty() {
        out = builtin_models(base).iter().map(|s| s.to_string()).collect();
    }
    out
}

/// Default model-list URL for URL mode; the form allows an override.
pub const MODELS_DEV_URL: &str = "https://models.dev/api.json";

/// models.dev provider ids backing each base; empty disables HTTP fetch.
pub fn models_dev_providers(base: CLIAgent) -> &'static [&'static str] {
    match base {
        CLIAgent::OpenCode | CLIAgent::OpenCode2 => &["opencode", "opencode-go"],
        CLIAgent::Claude => &["anthropic"],
        CLIAgent::Codex => &["openai"],
        CLIAgent::Gemini => &["google"],
        CLIAgent::Grok => &["xai"],
        CLIAgent::Qwen => &["alibaba"],
        CLIAgent::Kimi => &["moonshotai"],
        CLIAgent::Copilot => &["github-copilot"],
        _ => &[],
    }
}

/// Model source: local JSON file (`{flag, models[]}`, see
/// `docs/agents/models-load-file.md`) or HTTP(S) URL (models.dev-compatible).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelLoadSource {
    File(PathBuf),
    Url(String),
}

impl ModelLoadSource {
    /// Cache key for one loaded list (URL or file path).
    pub fn cache_key(&self) -> String {
        match self {
            ModelLoadSource::File(path) => format!("file:{}", path.display()),
            ModelLoadSource::Url(url) => format!("url:{url}"),
        }
    }
}

/// Effective load source from the form; `None` only when file mode lacks a path.
pub fn source_from_parts(from_file: bool, text: &str) -> Option<ModelLoadSource> {
    let trimmed = text.trim();
    if from_file {
        if trimmed.is_empty() {
            return None;
        }
        return Some(ModelLoadSource::File(PathBuf::from(trimmed)));
    }
    if trimmed.is_empty() {
        return Some(ModelLoadSource::Url(MODELS_DEV_URL.to_string()));
    }
    Some(ModelLoadSource::Url(trimmed.to_string()))
}

/// Live model lists for `base` from `source`, then local CLI fetch.
/// Failures read as the first error; callers keep their old list.
pub fn fetch_live_models(base: CLIAgent, source: &ModelLoadSource) -> Result<Vec<String>, String> {
    let mut first_err: Option<String> = None;
    let mut failed = |e: String| {
        if first_err.is_none() {
            first_err = Some(e);
        }
    };
    match source {
        ModelLoadSource::File(path) => match load_models_file(path, base) {
            Ok(models) => {
                write_models_cache(base, &source.cache_key(), &models);
                return Ok(models);
            }
            Err(e) => {
                failed(e);
                let cached = read_models_cache(base, &source.cache_key());
                if !cached.is_empty() {
                    return Ok(cached);
                }
            }
        },
        ModelLoadSource::Url(url) => {
            // Unmapped base + default URL would fetch megabytes to learn nothing.
            let skip_http = models_dev_providers(base).is_empty() && url == MODELS_DEV_URL;
            if !skip_http {
                match fetch_models_dev(url, base) {
                    Ok(models) => {
                        write_models_cache(base, &source.cache_key(), &models);
                        return Ok(models);
                    }
                    Err(e) => {
                        failed(e);
                        let cached = read_models_cache(base, &source.cache_key());
                        if !cached.is_empty() {
                            return Ok(cached);
                        }
                    }
                }
            }
        }
    }
    let local = fetch_models_local(base);
    if !local.is_empty() {
        return Ok(local);
    }
    Err(first_err.unwrap_or_else(|| format!("no models found for {}", base.slug())))
}

/// Live lists using only what the CLIs do themselves; failures read as empty.
fn fetch_models(base: CLIAgent) -> Vec<String> {
    fetch_live_models(base, &ModelLoadSource::Url(MODELS_DEV_URL.to_string())).unwrap_or_default()
}

/// What the installed CLIs report themselves; failures read as empty.
fn fetch_models_local(base: CLIAgent) -> Vec<String> {
    match base {
        CLIAgent::OpenCode | CLIAgent::OpenCode2 => fetch_opencode_models(),
        CLIAgent::Claude => read_claude_catalog_models(),
        _ => Vec::new(),
    }
}

/// Pull `url` and keep only `base`'s providers (sorted, de-duplicated).
/// Empty pick reads as an error so callers fall through.
fn fetch_models_dev(url: &str, base: CLIAgent) -> Result<Vec<String>, String> {
    let bytes = http_get_bytes(url)?;
    parse_models_dev(&bytes, base)
}

/// Parse a models.dev-compatible document for `base` into model ids
/// (`opencode*` keys gain their provider prefix; unknown fields ignored).
fn parse_models_dev(bytes: &[u8], base: CLIAgent) -> Result<Vec<String>, String> {
    const MAX_MODELS_DEV_BYTES: usize = 32 * 1024 * 1024;
    if bytes.len() > MAX_MODELS_DEV_BYTES {
        return Err(format!(
            "models list from {base} is larger than 32 MiB",
            base = base.slug()
        ));
    }
    let doc: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| format!("cannot parse model list: {e}"))?;
    let providers = models_dev_providers(base);
    let mut out: Vec<String> = Vec::new();
    for provider in providers {
        let Some(models) = doc.get(*provider).and_then(|p| p.get("models")) else {
            continue;
        };
        let Some(map) = models.as_object() else {
            continue;
        };
        for key in map.keys() {
            let id = key.trim();
            if id.is_empty() {
                continue;
            }
            if id.contains('/') {
                out.push(id.to_string());
            } else if matches!(*provider, "opencode" | "opencode-go") {
                out.push(format!("{provider}/{id}"));
            } else {
                out.push(id.to_string());
            }
        }
    }
    out.sort();
    out.dedup();
    if out.is_empty() {
        return Err(format!("no models listed for {}", base.slug()));
    }
    Ok(out)
}

#[cfg(feature = "remote-install")]
fn http_get_bytes(url: &str) -> Result<Vec<u8>, String> {
    use std::io::Read as _;
    const MAX_LIST_BYTES: u64 = 32 * 1024 * 1024;
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(8)))
        .timeout_connect(Some(std::time::Duration::from_secs(8)))
        .user_agent(concat!("tty7/", env!("CARGO_PKG_VERSION")))
        .build();
    let agent: ureq::Agent = agent.into();
    let response = agent
        .get(url)
        .call()
        .map_err(|e| format!("cannot fetch model list from {url}: {e}"))?;
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(format!("model list from {url} returned HTTP {status}"));
    }
    let mut body = response.into_body();
    let mut reader = body.as_reader().take(MAX_LIST_BYTES + 1);
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|e| format!("cannot read model list from {url}: {e}"))?;
    if bytes.len() as u64 > MAX_LIST_BYTES {
        return Err(format!("model list from {url} is larger than 32 MiB"));
    }
    Ok(bytes)
}

#[cfg(not(feature = "remote-install"))]
fn http_get_bytes(url: &str) -> Result<Vec<u8>, String> {
    Err(format!(
        "HTTP model fetch from {url} is unavailable in this build"
    ))
}

/// Load a model-list JSON file: hand-written `{flag, models[]}` or a
/// models.dev full/slice document (see `docs/agents/models-load-file.md`).
pub fn load_models_file(path: &Path, base: CLIAgent) -> Result<Vec<String>, String> {
    const MAX_FILE_BYTES: u64 = 1024 * 1024;
    const MAX_FILE_MODELS: usize = 2000;
    let meta = std::fs::metadata(path)
        .map_err(|e| format!("cannot read model file {}: {e}", path.display()))?;
    if meta.len() > MAX_FILE_BYTES {
        return Err(format!(
            "model file {} is larger than 1 MiB",
            path.display()
        ));
    }
    let bytes = std::fs::read(path)
        .map_err(|e| format!("cannot read model file {}: {e}", path.display()))?;
    let v: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|e| format!("cannot parse model file {}: {e}", path.display()))?;
    if v.get("models").is_none() {
        return parse_models_dev(&bytes, base)
            .map_err(|e| format!("model file {}: {e}", path.display()));
    }
    if let Some(flag) = v.get("flag")
        && !flag.is_null()
    {
        let ok = flag
            .as_str()
            .is_some_and(|f| f.trim().starts_with('-') && !f.trim().is_empty());
        if !ok {
            return Err(format!(
                "model file {} has an invalid `flag` (want e.g. \"--model\")",
                path.display()
            ));
        }
    }
    let models = v.get("models").and_then(|m| m.as_array()).ok_or_else(|| {
        format!(
            "model file {} needs a `models` string array (see docs/agents/models-load-file.md)",
            path.display()
        )
    })?;
    if models.len() > MAX_FILE_MODELS {
        return Err(format!(
            "model file {} lists more than {MAX_FILE_MODELS} models",
            path.display()
        ));
    }
    let mut out: Vec<String> = Vec::new();
    for (i, m) in models.iter().enumerate() {
        let Some(s) = m.as_str().map(str::trim).filter(|s| !s.is_empty()) else {
            return Err(format!(
                "model file {} has a non-string `models[{i}]`",
                path.display()
            ));
        };
        out.push(s.to_string());
    }
    out.sort();
    out.dedup();
    if out.is_empty() {
        return Err(format!("model file {} lists no models", path.display()));
    }
    Ok(out)
}

/// Per-base model cache beside `roles/` so a reopen works offline.
fn models_cache_dir() -> Option<PathBuf> {
    crate::core::config::config_dir_path().map(|d| d.join("models-cache"))
}

fn read_models_cache_in(dir: &Path, base: CLIAgent, source: &str) -> Vec<String> {
    let read = || -> Option<Vec<String>> {
        let text = std::fs::read_to_string(dir.join(format!("{}.json", base.slug()))).ok()?;
        let v: serde_json::Value = serde_json::from_str(&text).ok()?;
        if v.get("source").and_then(|s| s.as_str()) != Some(source) {
            return None;
        }
        v.get("models").and_then(|m| m.as_array()).map(|a| {
            a.iter()
                .filter_map(|e| e.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        })
    };
    read().unwrap_or_default()
}

fn write_models_cache_in(dir: &Path, base: CLIAgent, source: &str, models: &[String]) {
    if models.is_empty() {
        return;
    }
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let body = serde_json::json!({"source": source, "models": models});
    let text = serde_json::to_string(&body).unwrap_or_default();
    let _ = std::fs::write(dir.join(format!("{}.json", base.slug())), text);
}

/// Cached list for `base` under `source`'s key; empty when never cached.
pub fn read_models_cache(base: CLIAgent, source: &str) -> Vec<String> {
    models_cache_dir()
        .map(|d| read_models_cache_in(&d, base, source))
        .unwrap_or_default()
}

/// The form's last-used source per base, restored on reopen.
fn last_source_file(dir: &Path, base: CLIAgent) -> PathBuf {
    dir.join(format!("{}.source.json", base.slug()))
}

fn read_last_source_in(dir: &Path, base: CLIAgent) -> Option<(bool, String)> {
    let text = std::fs::read_to_string(last_source_file(dir, base)).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let from_file = v.get("from_file").and_then(|b| b.as_bool())?;
    let text = v
        .get("text")
        .and_then(|s| s.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())?
        .to_string();
    Some((from_file, text))
}

fn write_last_source_in(dir: &Path, base: CLIAgent, from_file: bool, text: &str) {
    let trimmed = text.trim();
    if from_file && trimmed.is_empty() {
        return;
    }
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let body = serde_json::json!({"from_file": from_file, "text": trimmed});
    let text = serde_json::to_string(&body).unwrap_or_default();
    let _ = std::fs::write(last_source_file(dir, base), text);
}

pub fn read_last_models_source(base: CLIAgent) -> Option<(bool, String)> {
    models_cache_dir().and_then(|d| read_last_source_in(&d, base))
}

pub fn write_last_models_source(base: CLIAgent, from_file: bool, text: &str) {
    if let Some(dir) = models_cache_dir() {
        write_last_source_in(&dir, base, from_file, text);
    }
}

fn write_models_cache(base: CLIAgent, source: &str, models: &[String]) {
    if let Some(dir) = models_cache_dir() {
        write_models_cache_in(&dir, base, source, models);
    }
}

/// `opencode models`, one `provider/model` per line. Runs synchronously —
/// callers only invoke it on base change or explicit refresh, never per
/// frame. Anything failing reads as empty.
///
/// v2 renamed the binary to `opencode2`; both names are tried in order so a
/// v2 install refreshes instead of silently falling back to the stale
/// built-in table.
fn fetch_opencode_models() -> Vec<String> {
    fetch_opencode_models_with_error().unwrap_or_default()
}

/// Same fetch with the failure reason kept, so the settings refresh button
/// can keep the old list *and* tell the user why it did not update.
pub fn fetch_opencode_models_with_error() -> Result<Vec<String>, String> {
    let Some(program) = find_opencode_program() else {
        return Err("no `opencode2` or `opencode` binary found on PATH".to_string());
    };
    run_models_command(&program)
}

/// Binary candidates in preference order: v2 first, then v1.
fn opencode_program_names() -> [&'static str; 2] {
    ["opencode2", "opencode"]
}

#[cfg(unix)]
fn is_executable(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.is_file()
        && std::fs::metadata(path)
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &std::path::Path) -> bool {
    path.is_file()
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&var) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        #[cfg(windows)]
        {
            for candidate in [dir.join(format!("{name}.exe")), dir.join(name)] {
                if is_executable(&candidate) {
                    return Some(candidate);
                }
            }
        }
        #[cfg(not(windows))]
        {
            let candidate = dir.join(name);
            if is_executable(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

/// First `opencode2`/`opencode` executable on `PATH`, else well-known install
/// locations (GUI launches often miss shell `PATH` entries). v2 first.
fn find_opencode_program() -> Option<PathBuf> {
    for name in opencode_program_names() {
        if let Some(found) = find_on_path(name) {
            return Some(found);
        }
    }
    let home = std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .map(PathBuf::from);
    if let Some(home) = home {
        for name in opencode_program_names() {
            for candidate in [
                home.join(".opencode").join("bin").join(name),
                home.join(".local").join("bin").join(name),
            ] {
                #[cfg(windows)]
                let candidate = candidate.with_extension("exe");
                if is_executable(&candidate) {
                    return Some(candidate);
                }
            }
        }
    }
    for name in opencode_program_names() {
        for candidate in [
            PathBuf::from("/usr/local/bin").join(name),
            PathBuf::from("/opt/homebrew/bin").join(name),
        ] {
            if is_executable(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

fn parse_models_output(stdout: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(stdout)
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// Run `<program> models` with a bounded wait so a hung CLI never freezes
/// the caller. `try_wait` polling is std-only (no extra dependency).
fn run_models_command(program: &std::path::Path) -> Result<Vec<String>, String> {
    let mut child = std::process::Command::new(program)
        .arg("models")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run `{}`: {e}", program.display()))?;
    let timeout = std::time::Duration::from_secs(8);
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let out = child
                    .wait_with_output()
                    .map_err(|e| format!("cannot read `{}` output: {e}", program.display()))?;
                if !status.success() {
                    let detail = String::from_utf8_lossy(&out.stderr).trim().to_string();
                    if detail.is_empty() {
                        return Err(format!("`{}` exited with {status}", program.display()));
                    }
                    return Err(format!("`{}` failed: {detail}", program.display()));
                }
                let models = parse_models_output(&out.stdout);
                if models.is_empty() {
                    return Err(format!("`{}` returned no models", program.display()));
                }
                return Ok(models);
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "`{}` timed out after {}s",
                        program.display(),
                        timeout.as_secs()
                    ));
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(e) => {
                let _ = child.kill();
                return Err(format!("cannot wait for `{}`: {e}", program.display()));
            }
        }
    }
}

/// Claude Code's local model-catalog cache (`$CLAUDE_CONFIG_DIR`, else
/// `~/.claude`): the model ids it lists, when it has fetched any. Reuses
/// the same roots the hooks installer honours.
fn read_claude_catalog_models() -> Vec<String> {
    let base = std::env::var_os("CLAUDE_CONFIG_DIR")
        .filter(|d| !d.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|h| !h.is_empty())
                .map(|h| std::path::PathBuf::from(h).join(".claude"))
        });
    let Some(dir) = base.map(|d| d.join("cache").join("model-catalog")) else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let bytes = std::fs::read(entry.path()).unwrap_or_default();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_default();
        if let Some(models) = json
            .pointer("/catalog/config/models")
            .and_then(|m| m.as_array())
        {
            out.extend(
                models
                    .iter()
                    .filter_map(|m| m.get("id")?.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string),
            );
        }
    }
    out.sort();
    out.dedup();
    out
}

/// A fresh slug for `want` that no slug in `existing` takes: `want` itself,
/// or `want-2`, `want-3`, … The Settings page mints these for new roles.
pub fn unique_slug(existing: &[String], want: &str) -> String {
    if !existing.iter().any(|s| s == want) {
        return want.to_string();
    }
    for n in 2..1000 {
        let cand = format!("{want}-{n}");
        if !existing.iter().any(|s| s == &cand) {
            return cand;
        }
    }
    format!("{want}-new")
}

/// `role` as the JSON `role.json` holds — the same shape [`load_roles`]
/// reads, so a save always loads back.
pub fn role_to_json(role: &AgentRole) -> serde_json::Value {
    serde_json::json!({
        "slug": role.slug,
        "name": role.name,
        "base": role.base.slug(),
        "description": role.description,
        "launch": role.launch,
        "model": role.model,
        "instructions": role.instructions,
        "starters": role.starters.iter().map(|s| {
            serde_json::json!({"label": s.label, "prompt": s.prompt})
        }).collect::<Vec<_>>(),
    })
}

/// Write `role` to `roles/<slug>/role.json` under `dir`, creating the
/// directory. Refuses a slug that fails [`slug_valid`].
pub fn save_role(dir: &Path, role: &AgentRole) -> std::io::Result<()> {
    if !slug_valid(&role.slug) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("invalid role slug {:?}", role.slug),
        ));
    }
    let sub = dir.join(&role.slug);
    std::fs::create_dir_all(&sub)?;
    let text = serde_json::to_string_pretty(&role_to_json(role)).map_err(std::io::Error::other)?;
    std::fs::write(sub.join("role.json"), text)?;
    log::info!("saved agent role {}", role.slug);
    Ok(())
}

/// Remove `roles/<slug>/` under `dir`. Missing slugs are already gone, so
/// that is success, not an error.
pub fn delete_role(dir: &Path, slug: &str) -> std::io::Result<()> {
    match std::fs::remove_dir_all(dir.join(slug)) {
        Ok(()) => {
            log::info!("deleted agent role {slug}");
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_role(dir: &Path, slug: &str, body: &str) {
        let file = dir.join(slug).join("role.json");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, body).unwrap();
    }

    #[test]
    fn bad_base_slug_skips_only_that_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let good = dir.path().join("good").join("role.json");
        std::fs::create_dir_all(good.parent().unwrap()).unwrap();
        std::fs::write(
            &good,
            r#"{"slug":"good","name":"Good","base":"claude","launch":"claude"}"#,
        )
        .unwrap();
        let bad = dir.path().join("bad").join("role.json");
        std::fs::create_dir_all(bad.parent().unwrap()).unwrap();
        std::fs::write(
            &bad,
            r#"{"slug":"bad","name":"Bad","base":"nope","launch":"nope"}"#,
        )
        .unwrap();
        let roles = load_roles(dir.path());
        assert_eq!(
            roles.iter().map(|r| r.slug.as_str()).collect::<Vec<_>>(),
            vec!["good"]
        );
    }

    #[test]
    fn slug_mismatch_skips_only_that_file() {
        let dir = tempfile::TempDir::new().unwrap();
        write_role(
            dir.path(),
            "good",
            r#"{"slug":"good","name":"Good","base":"claude","launch":"claude"}"#,
        );
        write_role(
            dir.path(),
            "other",
            r#"{"slug":"good","name":"Mismatch","base":"claude","launch":"claude"}"#,
        );
        let roles = load_roles(dir.path());
        assert_eq!(
            roles.iter().map(|r| r.slug.as_str()).collect::<Vec<_>>(),
            vec!["good"]
        );
    }

    #[test]
    fn empty_launch_means_the_bare_base_binary() {
        let dir = tempfile::TempDir::new().unwrap();
        write_role(
            dir.path(),
            "good",
            r#"{"slug":"good","name":"Good","base":"claude","launch":"claude"}"#,
        );
        write_role(
            dir.path(),
            "blank",
            r#"{"slug":"blank","name":"Blank","base":"claude","launch":"   "}"#,
        );
        let roles = load_roles(dir.path());
        assert_eq!(
            roles.iter().map(|r| r.slug.as_str()).collect::<Vec<_>>(),
            vec!["blank", "good"]
        );
        let blank = roles.iter().find(|r| r.slug == "blank").unwrap();
        assert_eq!(blank.launch, "");
        assert_eq!(role_launch_argv(blank), vec!["claude"]);
    }

    #[test]
    fn starters_truncate_to_three_and_drop_blanks() {
        let dir = tempfile::TempDir::new().unwrap();
        write_role(
            dir.path(),
            "good",
            r#"{"slug":"good","name":"Good","base":"claude","launch":"claude","starters":[{"label":"","prompt":"x"},{"label":"A","prompt":"a"},{"label":"B","prompt":"b"},{"label":"C","prompt":"c"},{"label":"D","prompt":"d"}]}"#,
        );
        let roles = load_roles(dir.path());
        assert_eq!(roles.len(), 1);
        assert_eq!(
            roles[0]
                .starters
                .iter()
                .map(|s| s.label.as_str())
                .collect::<Vec<_>>(),
            vec!["A", "B", "C"]
        );
    }

    #[test]
    fn slug_from_name_is_stable_and_unique() {
        assert_eq!(slug_from_name("Frontend Reviewer"), "frontend-reviewer");
        assert_eq!(slug_from_name("  !!  "), "role");
        let long = slug_from_name(&"a".repeat(100));
        assert!(slug_valid(&long), "minted slugs always save: {long}");
    }

    #[test]
    fn duplicate_names_get_numeric_suffix() {
        let existing = vec!["frontend-reviewer".to_string()];
        assert_eq!(
            unique_slug(&existing, "frontend-reviewer"),
            "frontend-reviewer-2"
        );
        assert_eq!(unique_slug(&existing, "other"), "other");
    }

    #[test]
    fn launch_argv_appends_model_flag() {
        let role = AgentRole {
            slug: "m".to_string(),
            name: "M".to_string(),
            base: CLIAgent::OpenCode,
            description: String::new(),
            launch: "opencode".to_string(),
            model: "gpt-x".to_string(),
            instructions: String::new(),
            starters: vec![],
        };
        assert_eq!(
            role_launch_argv(&role),
            vec!["opencode", "--model", "gpt-x"]
        );
        let bare = AgentRole {
            model: String::new(),
            ..role
        };
        assert_eq!(role_launch_argv(&bare), vec!["opencode"]);
    }

    #[test]
    fn blank_launch_argv_starts_with_the_base_binary() {
        let role = AgentRole {
            slug: "b".to_string(),
            name: "B".to_string(),
            base: CLIAgent::Claude,
            description: String::new(),
            launch: "   ".to_string(),
            model: String::new(),
            instructions: String::new(),
            starters: vec![],
        };
        assert_eq!(role_launch_argv(&role), vec!["claude"]);
    }

    #[test]
    fn blank_names_fall_back_to_slug() {
        let dir = tempfile::TempDir::new().unwrap();
        write_role(
            dir.path(),
            "good",
            r#"{"slug":"good","name":"   ","base":"claude","launch":"claude"}"#,
        );
        let roles = load_roles(dir.path());
        assert_eq!(roles.len(), 1);
        assert_eq!(roles[0].name, "good");
    }

    #[test]
    fn save_loads_back_what_was_saved() {
        let dir = tempfile::TempDir::new().unwrap();
        let role = AgentRole {
            slug: "frontend-reviewer".to_string(),
            name: "Frontend Reviewer".to_string(),
            base: CLIAgent::Claude,
            description: "Reviews frontend PRs".to_string(),
            launch: "claude --model opus".to_string(),
            model: "opus".to_string(),
            instructions: "Be terse.".to_string(),
            starters: vec![RoleStarter {
                label: "Review a PR".to_string(),
                prompt: "Review the open PR.".to_string(),
            }],
        };
        save_role(dir.path(), &role).unwrap();
        let roles = load_roles(dir.path());
        assert_eq!(roles, vec![role]);
    }

    #[test]
    fn saving_a_bad_slug_fails_and_deleting_a_ghost_succeeds() {
        let dir = tempfile::TempDir::new().unwrap();
        let bad = AgentRole {
            slug: "Nope!!".to_string(),
            name: "Nope".to_string(),
            base: CLIAgent::Claude,
            description: String::new(),
            launch: "claude".to_string(),
            model: String::new(),
            instructions: String::new(),
            starters: vec![],
        };
        assert!(save_role(dir.path(), &bad).is_err());
        assert!(delete_role(dir.path(), "ghost").is_ok());
    }

    #[test]
    fn opencode2_shares_opencodes_model_table() {
        assert_eq!(model_flag(CLIAgent::OpenCode2), Some("--model"));
        assert!(!model_choices_fast(CLIAgent::OpenCode2).is_empty());
        assert_eq!(
            model_choices_fast(CLIAgent::OpenCode2),
            model_choices_fast(CLIAgent::OpenCode)
        );
    }

    #[test]
    fn opencode_prefers_v2_binary_name() {
        assert_eq!(opencode_program_names(), ["opencode2", "opencode"]);
    }

    #[test]
    fn models_output_parses_one_per_line() {
        let out = b"opencode/gpt-5\n\n  opencode/claude-sonnet-4-5  \n";
        assert_eq!(
            parse_models_output(out),
            vec![
                "opencode/gpt-5".to_string(),
                "opencode/claude-sonnet-4-5".to_string()
            ]
        );
        assert!(parse_models_output(b"\n   \n").is_empty());
    }

    #[test]
    fn models_dev_providers_cover_the_mapped_bases() {
        assert_eq!(
            models_dev_providers(CLIAgent::OpenCode),
            &["opencode", "opencode-go"]
        );
        assert_eq!(
            models_dev_providers(CLIAgent::OpenCode2),
            models_dev_providers(CLIAgent::OpenCode)
        );
        assert!(!models_dev_providers(CLIAgent::Claude).is_empty());
        assert!(!models_dev_providers(CLIAgent::Codex).is_empty());
        assert!(!models_dev_providers(CLIAgent::Gemini).is_empty());
        assert!(!models_dev_providers(CLIAgent::Grok).is_empty());
        assert!(!models_dev_providers(CLIAgent::Qwen).is_empty());
        assert!(!models_dev_providers(CLIAgent::Kimi).is_empty());
        assert!(!models_dev_providers(CLIAgent::Copilot).is_empty());
        assert!(models_dev_providers(CLIAgent::Goose).is_empty());
        assert!(models_dev_providers(CLIAgent::Droid).is_empty());
    }

    #[test]
    fn models_dev_parses_only_mapped_providers() {
        let doc = serde_json::json!({
            "anthropic": {"models": {
                "claude-sonnet-4-5": {},
                "claude-haiku-4-5": {},
            }},
            "openai": {"models": {"gpt-5.1": {}}},
            "opencode": {"models": {"gpt-5": {}, "with/slash": {}}},
            "opencode-go": {"models": {"mimo-v2.6-pro": {}}},
        });
        let bytes = serde_json::to_vec(&doc).unwrap();
        assert_eq!(
            parse_models_dev(&bytes, CLIAgent::Claude).unwrap(),
            vec![
                "claude-haiku-4-5".to_string(),
                "claude-sonnet-4-5".to_string()
            ]
        );
        assert_eq!(
            parse_models_dev(&bytes, CLIAgent::OpenCode).unwrap(),
            vec![
                "opencode-go/mimo-v2.6-pro".to_string(),
                "opencode/gpt-5".to_string(),
                "with/slash".to_string(),
            ]
        );
        assert!(parse_models_dev(&bytes, CLIAgent::Goose).is_err());
        assert!(parse_models_dev(b"{}", CLIAgent::Claude).is_err());
        assert!(parse_models_dev(b"nope", CLIAgent::Claude).is_err());
    }

    #[test]
    fn load_models_file_accepts_flag_shape_and_rejects_junk() {
        let dir = tempfile::TempDir::new().unwrap();
        let good = dir.path().join("models.json");
        std::fs::write(
            &good,
            r#"{"flag": "--model", "models": ["b", "a", "a", " x "]}"#,
        )
        .unwrap();
        assert_eq!(
            load_models_file(&good, CLIAgent::Qwen).unwrap(),
            vec!["a".to_string(), "b".to_string(), "x".to_string()]
        );
        let no_flag = dir.path().join("noflag.json");
        std::fs::write(&no_flag, r#"{"models": ["m"]}"#).unwrap();
        assert_eq!(
            load_models_file(&no_flag, CLIAgent::Qwen).unwrap(),
            vec!["m".to_string()]
        );
        for (name, body) in [
            ("no-models.json", r#"{"flag": "--model"}"#),
            ("empty.json", r#"{"models": []}"#),
            ("mixed.json", r#"{"models": ["ok", 7]}"#),
            ("bad-flag.json", r#"{"flag": "model", "models": ["m"]}"#),
            ("bad-json.json", r#"{"models": "#),
        ] {
            let p = dir.path().join(name);
            std::fs::write(&p, body).unwrap();
            assert!(
                load_models_file(&p, CLIAgent::Qwen).is_err(),
                "{name} should fail"
            );
        }
        assert!(
            load_models_file(dir.path().join("missing.json").as_path(), CLIAgent::Qwen).is_err()
        );
    }

    #[test]
    fn load_models_file_accepts_a_models_dev_document() {
        let dir = tempfile::TempDir::new().unwrap();
        let doc = serde_json::json!({
            "anthropic": {"models": {"claude-sonnet-4-5": {}, "claude-haiku-4-5": {}}},
            "opencode": {"models": {"gpt-5": {}}},
            "deepinfra": {"models": {"x": {}}},
        });
        let p = dir.path().join("api.json");
        std::fs::write(&p, serde_json::to_vec(&doc).unwrap()).unwrap();
        assert_eq!(
            load_models_file(&p, CLIAgent::Claude).unwrap(),
            vec![
                "claude-haiku-4-5".to_string(),
                "claude-sonnet-4-5".to_string()
            ]
        );
        assert_eq!(
            load_models_file(&p, CLIAgent::OpenCode).unwrap(),
            vec!["opencode/gpt-5".to_string()]
        );
        // Slice with no mapped provider fails instead of blanking the list.
        let slim = dir.path().join("slim.json");
        std::fs::write(&slim, r#"{"deepinfra": {"models": {"x": {}}}}"#).unwrap();
        let err = load_models_file(&slim, CLIAgent::Claude).unwrap_err();
        assert!(err.contains("slim.json"), "unexpected error: {err}");
        // A present-but-broken `models` array keeps its precise error.
        let mixed = dir.path().join("mixed.json");
        std::fs::write(&mixed, r#"{"models": ["ok", 7]}"#).unwrap();
        let err = load_models_file(&mixed, CLIAgent::Claude).unwrap_err();
        assert!(err.contains("non-string"), "unexpected error: {err}");
    }

    #[test]
    fn models_cache_roundtrips_only_for_its_source() {
        let dir = tempfile::TempDir::new().unwrap();
        let models = vec!["a".to_string(), "b".to_string()];
        write_models_cache_in(dir.path(), CLIAgent::Qwen, "url:https://x", &models);
        assert_eq!(
            read_models_cache_in(dir.path(), CLIAgent::Qwen, "url:https://x"),
            models
        );
        assert!(read_models_cache_in(dir.path(), CLIAgent::Qwen, "url:https://y").is_empty());
        assert!(read_models_cache_in(dir.path(), CLIAgent::Claude, "url:https://x").is_empty());
        write_models_cache_in(dir.path(), CLIAgent::Qwen, "url:https://x", &[]);
        assert_eq!(
            read_models_cache_in(dir.path(), CLIAgent::Qwen, "url:https://x"),
            models,
            "empty writes must not clear the cache"
        );
    }

    #[test]
    fn source_parts_build_file_url_or_nothing() {
        assert!(source_from_parts(true, "   ").is_none());
        assert_eq!(
            source_from_parts(true, " ~/m.json "),
            Some(ModelLoadSource::File(PathBuf::from("~/m.json")))
        );
        assert_eq!(
            source_from_parts(false, ""),
            Some(ModelLoadSource::Url(MODELS_DEV_URL.to_string()))
        );
        assert_eq!(
            source_from_parts(false, "https://mirror/x"),
            Some(ModelLoadSource::Url("https://mirror/x".to_string()))
        );
    }

    #[test]
    fn last_source_roundtrips_per_base() {
        let dir = tempfile::TempDir::new().unwrap();
        assert!(read_last_source_in(dir.path(), CLIAgent::Qwen).is_none());
        write_last_source_in(dir.path(), CLIAgent::Qwen, true, "  ~/m.json  ");
        assert_eq!(
            read_last_source_in(dir.path(), CLIAgent::Qwen),
            Some((true, "~/m.json".to_string()))
        );
        assert!(read_last_source_in(dir.path(), CLIAgent::Claude).is_none());
        // Empty file text is not worth remembering.
        write_last_source_in(dir.path(), CLIAgent::Qwen, true, "   ");
        assert_eq!(
            read_last_source_in(dir.path(), CLIAgent::Qwen),
            Some((true, "~/m.json".to_string()))
        );
        std::fs::write(
            dir.path().join("qwen.source.json"),
            r#"{"from_file": "yes"}"#,
        )
        .unwrap();
        assert!(read_last_source_in(dir.path(), CLIAgent::Qwen).is_none());
    }

    #[test]
    fn unmapped_base_with_default_url_skips_http_and_fails_fast() {
        // No network, no CLI for goose: the skip means this errors from the
        // local chain, proving no HTTP was attempted for the default URL.
        let err = fetch_live_models(
            CLIAgent::Goose,
            &ModelLoadSource::Url(MODELS_DEV_URL.to_string()),
        )
        .unwrap_err();
        assert!(!err.is_empty());
    }

    #[test]
    fn missing_file_errors_with_the_path() {
        let dir = tempfile::TempDir::new().unwrap();
        let missing = dir.path().join("gone.json");
        let source = ModelLoadSource::File(missing);
        // No local fetcher for qwen either: surfaces the file error.
        let err = fetch_live_models(CLIAgent::Qwen, &source).unwrap_err();
        assert!(err.contains("gone.json"), "unexpected error: {err}");
    }

    #[test]
    fn launch_argv_skips_the_dropdown_when_the_line_names_the_flag() {
        let role = |launch: &str, model: &str| AgentRole {
            slug: "m".to_string(),
            name: "M".to_string(),
            base: CLIAgent::Claude,
            description: String::new(),
            launch: launch.to_string(),
            model: model.to_string(),
            instructions: String::new(),
            starters: vec![],
        };
        assert_eq!(
            role_launch_argv(&role("claude --model opus", "sonnet")),
            vec!["claude", "--model", "opus"]
        );
        assert_eq!(
            role_launch_argv(&role("claude --model=opus", "sonnet")),
            vec!["claude", "--model=opus"]
        );
        assert_eq!(
            role_launch_argv(&role("claude --verbose", "sonnet")),
            vec!["claude", "--verbose", "--model", "sonnet"]
        );
    }

    #[test]
    fn live_opencode_fetch_returns_models_when_cli_exists() {
        // Best-effort against the real CLI: skip when neither binary is
        // installed so CI without opencode stays green. Retry once: a cold
        // background service can transiently report an empty list.
        if find_opencode_program().is_none() {
            return;
        }
        let mut last = String::new();
        for _ in 0..2 {
            match fetch_opencode_models_with_error() {
                Ok(models) if !models.is_empty() => {
                    assert!(
                        models.iter().all(|m| m.contains('/')),
                        "expected provider/model entries, got {models:?}"
                    );
                    return;
                }
                Ok(models) => last = format!("empty list ({} entries)", models.len()),
                Err(e) => last = e,
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        panic!("installed opencode CLI should list models: {last}");
    }
}
