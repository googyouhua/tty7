//! Where a Markdown preview link points when it names a local file.
//!
//! Pure path logic: no gpui, no host handles, so the whole edge-case matrix
//! is pinned by ordinary unit tests below. Opening the answer (editor, tree,
//! browser, notification) lives with the preview in `super`.

use std::path::{Component, Path, PathBuf};

/// What clicking a preview link should do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MarkdownLinkAction {
    /// A local path, shown the way file links elsewhere are.
    OpenLocal(PathBuf),
    /// Anything the resolver does not read as local: handed to the OS.
    OpenExternal(String),
    /// A `file:` URL no file path can be read from: ignored, with a notice.
    InvalidFileUrl(String),
}

/// Decides what a click on `url` — written in the file at `base` — does.
///
/// A preview of a file on another machine has no local files to name, so off
/// that machine everything keeps the old behaviour.
pub(crate) fn classify_markdown_link(
    host_is_local: bool,
    base: &Path,
    url: &str,
) -> MarkdownLinkAction {
    let raw = url.trim();
    if !host_is_local {
        return MarkdownLinkAction::OpenExternal(raw.to_string());
    }
    match resolve_markdown_link(base, raw) {
        Some(path) => MarkdownLinkAction::OpenLocal(path),
        None if is_file_url(raw) => MarkdownLinkAction::InvalidFileUrl(raw.to_string()),
        None => MarkdownLinkAction::OpenExternal(raw.to_string()),
    }
}

/// Whether `url` claims the `file:` scheme.
pub(crate) fn is_file_url(url: &str) -> bool {
    matches!(scheme_of(url.trim()), Some(scheme) if scheme.eq_ignore_ascii_case("file"))
}

/// Reads `url` as a local file path, resolved against the file it was
/// written in.
///
/// Relative links start from that file's directory (`./other.md` beside
/// `/repo/docs/a.md` is `/repo/docs/other.md`); `file:` URLs and `/absolute`
/// paths stand on their own. `#fragments` and `?queries` never name a file
/// and are peeled first.
///
/// Returns `None` for anything that is not a local path: external targets
/// (`http:`/`https:`/`mailto:`, or any other scheme), bare `#fragment`s,
/// empties, and `file:` URLs no path can be read from.
pub(crate) fn resolve_markdown_link(base: &Path, url: &str) -> Option<PathBuf> {
    let mut text = url.trim();
    if text.is_empty() || text.starts_with('#') {
        return None;
    }
    match scheme_of(text) {
        // Through the URL parser, which also undoes any percent-encoding.
        Some(scheme) if scheme.eq_ignore_ascii_case("file") => {
            return url::Url::parse(text).ok()?.to_file_path().ok();
        }
        // `https://…`, `mailto:…`, any other scheme: not a file.
        Some(_) => return None,
        None => {}
    }
    if let Some(hash) = text.find('#') {
        text = text[..hash].trim();
    }
    if let Some(query) = text.find('?') {
        text = text[..query].trim();
    }
    if text.is_empty() {
        return None;
    }
    let target = Path::new(text);
    if target.is_absolute() {
        Some(normalize_lexically(target))
    } else {
        let dir = base.parent().unwrap_or_else(|| Path::new(""));
        Some(normalize_lexically(&dir.join(target)))
    }
}

/// The URI scheme of `text` (`http` in `http://…`), if it has one.
///
/// A scheme is `ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )` followed by `:` —
/// anything else carrying a colon (`./a:b.md`, `localhost:8080`) is a path,
/// not a URL.
fn scheme_of(text: &str) -> Option<&str> {
    let scheme = text.split(':').next()?;
    if text.len() <= scheme.len() {
        return None;
    }
    let mut bytes = scheme.bytes();
    if !bytes.next().is_some_and(|b| b.is_ascii_alphabetic()) {
        return None;
    }
    if !bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.')) {
        return None;
    }
    // A lone letter driving a path (`C:/…`, `C:\…`) is a Windows drive, not
    // a URI scheme.
    if scheme.len() == 1
        && text.as_bytes()[scheme.len() + 1..]
            .first()
            .is_some_and(|b| *b == b'/' || *b == b'\\')
    {
        return None;
    }
    Some(scheme)
}

/// Joins away `.` and `..` without touching the filesystem: the target may
/// not exist yet (a link to a file about to be created), and resolving
/// symlinks here would disagree with the open that follows.
fn normalize_lexically(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::RootDir => out.push(component.as_os_str()),
            Component::CurDir => {}
            // Past the root there is nowhere to go: `PathBuf::pop` says so
            // and the `..` is dropped, the way the desktop opens it too.
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(part) => out.push(part),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> PathBuf {
        PathBuf::from("/repo/docs/a.md")
    }

    #[test]
    fn relative_links_start_from_the_writing_files_directory() {
        assert_eq!(
            resolve_markdown_link(&base(), "./other.md"),
            Some(PathBuf::from("/repo/docs/other.md"))
        );
        assert_eq!(
            resolve_markdown_link(&base(), "other.md"),
            Some(PathBuf::from("/repo/docs/other.md"))
        );
        assert_eq!(
            resolve_markdown_link(&base(), "sub/note.md"),
            Some(PathBuf::from("/repo/docs/sub/note.md"))
        );
    }

    #[test]
    fn parent_links_climb_and_dots_fold_away() {
        assert_eq!(
            resolve_markdown_link(&base(), "../config.json"),
            Some(PathBuf::from("/repo/config.json"))
        );
        assert_eq!(
            resolve_markdown_link(&base(), "./sub/../other.md"),
            Some(PathBuf::from("/repo/docs/other.md"))
        );
        assert_eq!(
            resolve_markdown_link(&base(), "/a/b/../c.md"),
            Some(PathBuf::from("/a/c.md"))
        );
    }

    #[test]
    fn absolute_and_file_urls_stand_on_their_own() {
        assert_eq!(
            resolve_markdown_link(&base(), "/var/log/app.log"),
            Some(PathBuf::from("/var/log/app.log"))
        );
        assert_eq!(
            resolve_markdown_link(&base(), "file:///tmp/x.md"),
            Some(PathBuf::from("/tmp/x.md"))
        );
    }

    #[test]
    fn fragments_queries_and_surrounding_space_are_not_the_file() {
        assert_eq!(
            resolve_markdown_link(&base(), "./other.md#标题"),
            Some(PathBuf::from("/repo/docs/other.md"))
        );
        assert_eq!(
            resolve_markdown_link(&base(), "./other.md?x=1#t"),
            Some(PathBuf::from("/repo/docs/other.md"))
        );
        assert_eq!(
            resolve_markdown_link(&base(), "  ./other.md  "),
            Some(PathBuf::from("/repo/docs/other.md"))
        );
    }

    #[test]
    fn external_targets_and_bare_anchors_are_not_files() {
        for url in [
            "https://example.com",
            "http://example.com/x.md",
            "HTTP://EXAMPLE.COM",
            "mailto:someone@example.com",
            "MAILTO:someone@example.com",
            "ftp://example.com/x",
            "#section",
            "",
            "   ",
            "?only-a-query",
        ] {
            assert_eq!(resolve_markdown_link(&base(), url), None, "url {url:?}");
        }
    }

    #[test]
    fn an_unreadable_file_url_is_not_a_file() {
        // A `file:` URL on another host has no path here.
        assert_eq!(
            resolve_markdown_link(&base(), "file://other-host/x.md"),
            None
        );
        assert!(is_file_url("file:///tmp/x.md"));
        assert!(is_file_url("FILE:///tmp/x.md"));
        assert!(is_file_url("file://other-host/x.md"));
        assert!(!is_file_url("./other.md"));
        assert!(!is_file_url("https://example.com"));
    }

    #[test]
    fn a_colon_does_not_make_everything_a_scheme() {
        // `./a:b.md` is a filename with a colon in it.
        assert_eq!(
            resolve_markdown_link(&base(), "./a:b.md"),
            Some(PathBuf::from("/repo/docs/a:b.md"))
        );
    }

    #[test]
    fn windows_drive_paths_are_not_schemes() {
        assert_eq!(scheme_of("C:/docs/other.md"), None);
        assert_eq!(scheme_of("C:\\docs\\other.md"), None);
        // …so they resolve instead of reading as external targets.
        // `std::path` reads them per platform: absolute on Windows, a
        // colon-bearing relative name anywhere else (both honest).
        #[cfg(windows)]
        assert_eq!(
            resolve_markdown_link(&base(), "C:/docs/other.md"),
            Some(PathBuf::from("C:/docs/other.md"))
        );
        #[cfg(not(windows))]
        assert_eq!(
            resolve_markdown_link(&base(), "C:/docs/other.md"),
            Some(PathBuf::from("/repo/docs/C:/docs/other.md"))
        );
        assert!(resolve_markdown_link(&base(), "C:\\docs\\other.md").is_some());
    }

    #[test]
    fn climbing_past_the_root_clamps_there() {
        assert_eq!(
            resolve_markdown_link(Path::new("/a.md"), "../../x.md"),
            Some(PathBuf::from("/x.md"))
        );
    }

    #[test]
    fn classification_keeps_remote_hosts_and_valid_urls_apart() {
        use MarkdownLinkAction::*;
        assert_eq!(
            classify_markdown_link(true, &base(), "./other.md"),
            OpenLocal(PathBuf::from("/repo/docs/other.md"))
        );
        assert_eq!(
            classify_markdown_link(true, &base(), "file:///tmp/x.md"),
            OpenLocal(PathBuf::from("/tmp/x.md"))
        );
        assert_eq!(
            classify_markdown_link(true, &base(), "https://example.com"),
            OpenExternal("https://example.com".to_string())
        );
        assert_eq!(
            classify_markdown_link(true, &base(), "#section"),
            OpenExternal("#section".to_string())
        );
        assert_eq!(
            classify_markdown_link(true, &base(), "file://other-host/x.md"),
            InvalidFileUrl("file://other-host/x.md".to_string())
        );
        // Off the local machine nothing resolves locally.
        assert_eq!(
            classify_markdown_link(false, &base(), "./other.md"),
            OpenExternal("./other.md".to_string())
        );
    }
}
