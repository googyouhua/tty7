//! Where a Markdown preview link points when it names a local file.
//!
//! Pure path logic: no gpui, no host handles, so the whole edge-case matrix
//! is pinned by ordinary unit tests below. Opening the answer (editor, tree,
//! browser, notification) lives with the preview in `super`.
//!
//! Deliberately platform-independent: everything below reads `/`-separated
//! text and only builds a `PathBuf` at the end, so the same link resolves
//! the same way on every OS. `std::path`'s own answers move underfoot
//! instead — `is_absolute` says `false` for `/home/me` on Windows, and
//! `Url::to_file_path` turns `file://other-host/x` into a live UNC path
//! there while refusing drive-less `file:///tmp/x` — which is also why this
//! module never calls either (see `check-host-boundary.sh`).

use std::path::{Path, PathBuf};

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
/// `/repo/docs/a.md` is `/repo/docs/other.md`); `file:` URLs and absolute
/// paths (`/x`, `C:/x`) stand on their own. `#fragments` and `?queries`
/// never name a file and are peeled first.
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
        // Parsed explicitly below rather than through `Url::to_file_path`,
        // whose answer is platform-dependent (see the module docs).
        Some(scheme) if scheme.eq_ignore_ascii_case("file") => {
            return file_url_to_path(text);
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
    // Backslashes separate path segments on Windows and are ordinary
    // filename characters everywhere else.
    #[cfg(windows)]
    let text = &text.replace('\\', "/");
    if is_absolute_str(text) {
        Some(join_forward_slashes("", text))
    } else {
        let dir = base.parent().map(forward_slashes).unwrap_or_default();
        Some(join_forward_slashes(&dir, text))
    }
}

/// Reads a `file:` URL as a path, without `Url::to_file_path`.
///
/// `file:///tmp/x.md` and `file://localhost/tmp/x.md` name `/tmp/x.md` on
/// every platform. Any other host names a share only Windows can reach, so
/// elsewhere it reads as unreadable; on Windows it becomes a UNC path.
fn file_url_to_path(text: &str) -> Option<PathBuf> {
    let url = url::Url::parse(text).ok()?;
    if !url.scheme().eq_ignore_ascii_case("file") {
        return None;
    }
    let host = url.host_str().unwrap_or("");
    if host.is_empty() || host.eq_ignore_ascii_case("localhost") {
        let path = percent_decode(url.path())?;
        // `file:///C:/x.md` carries its drive behind one leading slash.
        #[cfg(windows)]
        let path = strip_drive_slash(&path).unwrap_or(path);
        Some(PathBuf::from(path))
    } else {
        #[cfg(windows)]
        {
            let rest = percent_decode(url.path())?;
            let rest = rest.strip_prefix('/').unwrap_or(&rest).replace('/', "\\");
            Some(PathBuf::from(format!("\\\\{host}\\{rest}")))
        }
        #[cfg(not(windows))]
        {
            None
        }
    }
}

/// Percent-decodes `text`, refusing it when the escapes are not UTF-8.
fn percent_decode(text: &str) -> Option<String> {
    percent_encoding::percent_decode_str(text)
        .decode_utf8()
        .ok()
        .map(|decoded| decoded.into_owned())
}

/// Whether `text` names an absolute path: a leading `/`, or a Windows drive
/// (`C:/…`, `C:\…`, `C:`-relative aside — a bare drive without a separator
/// is a relative name everywhere).
fn is_absolute_str(text: &str) -> bool {
    text.starts_with('/') || is_drive_absolute(text)
}

/// Whether `text` starts with a Windows drive plus separator (`C:/`, `D:\`).
/// Read uniformly on every platform: on Windows it is genuinely absolute,
/// and elsewhere the joined result simply names nothing that exists.
fn is_drive_absolute(text: &str) -> bool {
    let mut bytes = text.bytes();
    matches!(bytes.next(), Some(b) if b.is_ascii_alphabetic())
        && matches!(bytes.next(), Some(b':'))
        && matches!(bytes.next(), Some(b'/' | b'\\'))
}

/// Strips the one leading slash from `/C:/…`, for `file:` URLs on Windows.
#[cfg(windows)]
fn strip_drive_slash(path: &str) -> Option<String> {
    let stripped = path.strip_prefix('/')?;
    is_drive_absolute(stripped).then(|| stripped.to_string())
}

/// Joins `dir` and `rel` on `/` separators and folds away `.` and `..`
/// lexically: the target may not exist yet (a link to a file about to be
/// created), and touching the filesystem here would disagree with the open
/// that follows. Climbing past the root clamps there.
fn join_forward_slashes(dir: &str, rel: &str) -> PathBuf {
    // Split off the root: `//server/share` (UNC), `/abs`, `C:/`, or none
    // (a relative link, resolved against `dir`). `protected` counts the
    // leading segments `..` may never pop past (the UNC share; `/` and
    // drives clamp at zero parts instead).
    let (root, rest, protected) =
        if let Some(stripped) = rel.strip_prefix("//").filter(|s| !s.starts_with('/')) {
            ("//", stripped, 2)
        } else if let Some(stripped) = rel.strip_prefix('/') {
            ("/", stripped, 0)
        } else if is_drive_absolute(rel) {
            let (head, tail) = rel.split_at(2);
            let tail = tail.strip_prefix(['/', '\\']).unwrap_or(tail);
            (head, tail, 0)
        } else {
            ("", rel, 0)
        };
    let mut parts: Vec<&str> = Vec::new();
    // A relative link inherits its absoluteness from the base dir.
    let rooted = !root.is_empty() || dir.starts_with('/');
    if root.is_empty() && !dir.is_empty() {
        parts.extend(dir.split('/').filter(|p| !p.is_empty()));
    }
    for part in rest.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." if parts.len() > protected => {
                parts.pop();
            }
            // Past the root (or the UNC share) there is nowhere to go:
            // drop the `..`, the way the desktop opens it too.
            ".." => {}
            _ => parts.push(part),
        }
    }
    // Reattach the root: `//` stays doubled, a drive takes its slash,
    // and a relative link inherits `/` from an absolute base dir. With
    // nothing left at all, an absolute link still names the root while a
    // relative one with an empty base stays empty.
    let mut out = if root == "//" {
        String::from("//")
    } else if root.len() == 2 {
        format!("{root}/")
    } else if rooted {
        String::from("/")
    } else {
        String::new()
    };
    out.push_str(&parts.join("/"));
    PathBuf::from(out)
}

/// Renders `path` with `/` separators for the string-based join above.
fn forward_slashes(path: &Path) -> String {
    // `to_string_lossy` cannot fail usefully here: a base file with a
    // non-Unicode name still resolves its sibling links by the same bytes
    // the join below reattaches.
    path.to_string_lossy().replace('\\', "/")
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
        // A `file:` URL on another host names a share only Windows can
        // reach: elsewhere it reads as unreadable.
        #[cfg(windows)]
        assert_eq!(
            resolve_markdown_link(&base(), "file://other-host/x.md"),
            Some(PathBuf::from("\\\\other-host\\x.md"))
        );
        #[cfg(not(windows))]
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
    fn file_urls_decode_escapes_and_localhost() {
        assert_eq!(
            resolve_markdown_link(&base(), "file:///tmp/a%20b.md"),
            Some(PathBuf::from("/tmp/a b.md"))
        );
        assert_eq!(
            resolve_markdown_link(&base(), "file://localhost/tmp/x.md"),
            Some(PathBuf::from("/tmp/x.md"))
        );
        // `file:///C:/x.md` carries its drive behind one leading slash.
        #[cfg(windows)]
        assert_eq!(
            resolve_markdown_link(&base(), "file:///C:/x.md"),
            Some(PathBuf::from("C:/x.md"))
        );
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
        // …so they resolve as absolute paths on every platform, not as
        // external targets: the string rules below do not move with the OS.
        assert_eq!(
            resolve_markdown_link(&base(), "C:/docs/other.md"),
            Some(PathBuf::from("C:/docs/other.md"))
        );
        assert_eq!(
            resolve_markdown_link(&base(), "C:\\docs\\other.md"),
            Some(PathBuf::from("C:/docs/other.md"))
        );
    }

    #[test]
    fn climbing_past_the_root_clamps_there() {
        assert_eq!(
            resolve_markdown_link(Path::new("/a.md"), "../../x.md"),
            Some(PathBuf::from("/x.md"))
        );
    }

    #[test]
    fn unc_roots_survive_the_fold() {
        assert_eq!(
            resolve_markdown_link(&base(), "//server/share/x.md"),
            Some(PathBuf::from("//server/share/x.md"))
        );
        // `..` never pops past the share, like `PathBuf::pop` refusing
        // below `\\server\share`.
        assert_eq!(
            resolve_markdown_link(&base(), "//server/share/../x.md"),
            Some(PathBuf::from("//server/share/x.md"))
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
        // A hosted `file:` URL matches the resolver above: a UNC path on
        // Windows, unreadable anywhere else.
        #[cfg(windows)]
        assert_eq!(
            classify_markdown_link(true, &base(), "file://other-host/x.md"),
            OpenLocal(PathBuf::from("\\\\other-host\\x.md"))
        );
        #[cfg(not(windows))]
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
