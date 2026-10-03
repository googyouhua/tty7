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
    let s = s.trim_matches('-').to_string();
    if s.is_empty() { "role".to_string() } else { s }
}

pub fn roles_dir() -> Option<PathBuf> {
    crate::core::config::config_dir_path().map(|d| d.join("roles"))
}

fn load_one(dir: &Path, slug: &str) -> Option<AgentRole> {
    let text = std::fs::read_to_string(dir.join("role.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let file_slug = v.get("slug").and_then(|s| s.as_str()).unwrap_or("");
    if file_slug != slug || !slug_valid(file_slug) {
        return None;
    }
    let base = v
        .get("base")
        .and_then(|s| s.as_str())
        .and_then(CLIAgent::from_slug)?;
    let launch = v
        .get("launch")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if launch.is_empty() {
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
}
