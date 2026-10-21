//! Runtime lookup of the gettext catalog directory.

use std::path::{Path, PathBuf};

/// gettext domain of this app. Distinct from upstream's `rog-control-center`
/// so that upstream's catalog is never picked up.
pub const GETTEXT_DOMAIN: &str = "rog-control-center-modern";

/// Pick the locale directory to bind.
///
/// An explicit override (`$ROGCC_LOCALE_DIR`) is honoured as given; every
/// other candidate is only used when `has_catalog` says it holds our catalog.
pub fn resolve_locale_dir(
    override_dir: Option<PathBuf>,
    candidates: &[PathBuf],
    has_catalog: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    if override_dir.is_some() {
        return override_dir;
    }
    candidates.iter().find(|dir| has_catalog(dir)).cloned()
}

/// Candidate directories in lookup order, after the `$ROGCC_LOCALE_DIR`
/// override: `<exe_dir>/../share/locale`, `$XDG_DATA_HOME/locale`
/// (default `~/.local/share/locale`), `/usr/share/locale`.
pub fn locale_candidates(
    exe_dir: Option<&Path>,
    xdg_data_home: Option<PathBuf>,
    home: Option<PathBuf>,
) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(exe_dir) = exe_dir {
        out.push(exe_dir.join("..").join("share").join("locale"));
    }
    let data_home = xdg_data_home
        .filter(|p| p.is_absolute())
        .or_else(|| home.map(|h| h.join(".local").join("share")));
    if let Some(data_home) = data_home {
        out.push(data_home.join("locale"));
    }
    out.push(PathBuf::from("/usr/share/locale"));
    out
}

/// Whether `dir` holds at least one `<lang>/LC_MESSAGES/<domain>.mo`.
pub fn dir_has_catalog(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries.flatten().any(|e| {
        e.path()
            .join("LC_MESSAGES")
            .join(format!("{GETTEXT_DOMAIN}.mo"))
            .is_file()
    })
}

/// Resolve the locale directory from the process environment.
pub fn resolve_from_env() -> Option<PathBuf> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf));
    let candidates = locale_candidates(
        exe_dir.as_deref(),
        std::env::var_os("XDG_DATA_HOME").map(PathBuf::from),
        dirs::home_dir(),
    );
    let override_dir = std::env::var_os("ROGCC_LOCALE_DIR")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from);
    resolve_locale_dir(override_dir, &candidates, dir_has_catalog)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dirs(list: &[&str]) -> Vec<PathBuf> {
        list.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn override_wins_even_without_catalog() {
        let chosen = resolve_locale_dir(
            Some(PathBuf::from("/custom")),
            &dirs(&[
                "/a", "/b",
            ]),
            |_| true,
        );
        assert_eq!(chosen, Some(PathBuf::from("/custom")));
    }

    #[test]
    fn first_candidate_with_catalog_is_chosen() {
        let chosen = resolve_locale_dir(
            None,
            &dirs(&[
                "/a", "/b", "/c",
            ]),
            |d| d == Path::new("/b") || d == Path::new("/c"),
        );
        assert_eq!(chosen, Some(PathBuf::from("/b")));
    }

    #[test]
    fn none_when_no_candidate_has_catalog() {
        assert_eq!(resolve_locale_dir(None, &dirs(&["/a"]), |_| false), None);
    }

    #[test]
    fn candidates_follow_documented_order() {
        let c = locale_candidates(
            Some(Path::new("/opt/app/bin")),
            Some(PathBuf::from("/xdg")),
            Some(PathBuf::from("/home/u")),
        );
        assert_eq!(
            c,
            dirs(&[
                "/opt/app/bin/../share/locale",
                "/xdg/locale",
                "/usr/share/locale"
            ])
        );
    }

    #[test]
    fn xdg_data_home_defaults_to_home_local_share() {
        let c = locale_candidates(None, None, Some(PathBuf::from("/home/u")));
        assert_eq!(
            c,
            dirs(&[
                "/home/u/.local/share/locale",
                "/usr/share/locale"
            ])
        );
    }
}
