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
    if launch.is_empty() {
        log::warn!("ignoring role at {}: empty launch command", dir.display());
        return None;
    }
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
    crate::core::cli_agent::launch_program(&role.launch)
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
    let text = serde_json::to_string_pretty(&role_to_json(role))
        .map_err(std::io::Error::other)?;
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
    fn empty_launch_skips_only_that_file() {
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
            vec!["good"]
        );
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
    fn save_loads_back_what_was_saved() {        let dir = tempfile::TempDir::new().unwrap();
        let role = AgentRole {
            slug: "frontend-reviewer".to_string(),
            name: "Frontend Reviewer".to_string(),
            base: CLIAgent::Claude,
            description: "Reviews frontend PRs".to_string(),
            launch: "claude --model opus".to_string(),
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
            instructions: String::new(),
            starters: vec![],
        };
        assert!(save_role(dir.path(), &bad).is_err());
        assert!(delete_role(dir.path(), "ghost").is_ok());
    }
}
