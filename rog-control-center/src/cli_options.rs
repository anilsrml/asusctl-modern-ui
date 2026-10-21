use argh::FromArgs;

#[derive(Default, FromArgs)]
/// ROG Control Center
pub struct CliStart {
    /// start fullscreen, if used the option is saved
    #[argh(switch)]
    pub fullscreen: bool,
    /// fullscreen width
    #[argh(option, default = "0")]
    pub width_fullscreen: u32,
    /// fullscreen height
    #[argh(option, default = "0")]
    pub height_fullscreen: u32,
    /// start windowed, if used the option is saved
    #[argh(switch)]
    pub windowed: bool,
    /// show program version number
    #[argh(switch)]
    pub version: bool,
    /// start in background (UI closed)
    #[argh(switch)]
    pub background: bool,
    /// indicate that the app was launched via autostart
    #[argh(switch)]
    pub autostart: bool,
    /// set board name for testing, this will make ROGCC show only the keyboard page
    #[argh(option)]
    pub board_name: Option<String>,
    /// put ROGCC in layout viewing mode - this is helpful for finding existing layouts that might match your laptop
    #[argh(switch)]
    pub layout_viewing: bool,
    /// open on this page: overview, keyboard, anime, slash, fans, gpu, battery, settings, about
    #[argh(option)]
    pub page: Option<String>,
}

/// Sidebar order of the pages; must match the model in ui/main_window.slint
const PAGES: [&str; 9] = [
    "overview", "keyboard", "anime", "slash", "fans", "gpu", "battery", "settings", "about",
];

/// Sidebar index of a page name given to `--page`
pub fn page_index(name: &str) -> Option<i32> {
    PAGES
        .iter()
        .position(|p| p.eq_ignore_ascii_case(name))
        .map(|i| i as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_names_map_to_sidebar_indexes() {
        assert_eq!(page_index("overview"), Some(0));
        assert_eq!(page_index("keyboard"), Some(1));
        assert_eq!(page_index("fans"), Some(4));
        assert_eq!(page_index("gpu"), Some(5));
        assert_eq!(page_index("battery"), Some(6));
        assert_eq!(page_index("about"), Some(8));
    }

    #[test]
    fn page_names_ignore_case_and_unknown_is_none() {
        assert_eq!(page_index("GPU"), Some(5));
        assert_eq!(page_index("system"), None);
        assert_eq!(page_index(""), None);
    }
}
