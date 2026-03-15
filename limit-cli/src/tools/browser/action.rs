//! Browser action enumeration for type-safe action dispatch
//!
//! Provides a comprehensive enum of all supported browser actions
//! with bidirectional string conversion via FromStr and as_str().

use limit_agent::error::AgentError;
use std::str::FromStr;

/// All supported browser automation actions
///
/// This enum provides type-safe dispatch of browser operations,
/// ensuring exhaustive handling at compile time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BrowserAction {
    // ========================================
    // Navigation
    // ========================================
    Open,
    Close,
    Snapshot,
    Screenshot,
    Back,
    Forward,
    Reload,

    // ========================================
    // Interaction
    // ========================================
    Click,
    Fill,
    Type,
    Press,
    Hover,
    Select,
    Dblclick,
    Focus,
    Check,
    Uncheck,
    Scrollintoview,
    Drag,
    Upload,
    Pdf,

    // ========================================
    // Query
    // ========================================
    Get,
    GetAttr,
    GetCount,
    GetBox,
    GetStyles,

    // ========================================
    // Wait
    // ========================================
    Wait,
    WaitForText,
    WaitForUrl,
    WaitForLoad,
    WaitForDownload,
    WaitForFn,
    WaitForState,

    // ========================================
    // State
    // ========================================
    Find,
    Scroll,
    Is,
    Download,

    // ========================================
    // Tabs
    // ========================================
    TabList,
    TabNew,
    TabClose,
    TabSelect,

    // ========================================
    // Dialog
    // ========================================
    DialogAccept,
    DialogDismiss,

    // ========================================
    // Storage & Network
    // ========================================
    Cookies,
    CookiesSet,
    StorageGet,
    StorageSet,
    NetworkRequests,

    // ========================================
    // Settings
    // ========================================
    SetViewport,
    SetDevice,
    SetGeo,

    // ========================================
    // Eval
    // ========================================
    Eval,
}

impl FromStr for BrowserAction {
    type Err = AgentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            // Navigation
            "open" => Ok(Self::Open),
            "close" => Ok(Self::Close),
            "snapshot" => Ok(Self::Snapshot),
            "screenshot" => Ok(Self::Screenshot),
            "back" => Ok(Self::Back),
            "forward" => Ok(Self::Forward),
            "reload" => Ok(Self::Reload),

            // Interaction
            "click" => Ok(Self::Click),
            "fill" => Ok(Self::Fill),
            "type" => Ok(Self::Type),
            "press" => Ok(Self::Press),
            "hover" => Ok(Self::Hover),
            "select" => Ok(Self::Select),
            "dblclick" => Ok(Self::Dblclick),
            "focus" => Ok(Self::Focus),
            "check" => Ok(Self::Check),
            "uncheck" => Ok(Self::Uncheck),
            "scrollintoview" => Ok(Self::Scrollintoview),
            "drag" => Ok(Self::Drag),
            "upload" => Ok(Self::Upload),
            "pdf" => Ok(Self::Pdf),

            // Query
            "get" => Ok(Self::Get),
            "get_attr" => Ok(Self::GetAttr),
            "get_count" => Ok(Self::GetCount),
            "get_box" => Ok(Self::GetBox),
            "get_styles" => Ok(Self::GetStyles),

            // Wait
            "wait" => Ok(Self::Wait),
            "wait_for_text" => Ok(Self::WaitForText),
            "wait_for_url" => Ok(Self::WaitForUrl),
            "wait_for_load" => Ok(Self::WaitForLoad),
            "wait_for_download" => Ok(Self::WaitForDownload),
            "wait_for_fn" => Ok(Self::WaitForFn),
            "wait_for_state" => Ok(Self::WaitForState),

            // State
            "find" => Ok(Self::Find),
            "scroll" => Ok(Self::Scroll),
            "is" => Ok(Self::Is),
            "download" => Ok(Self::Download),

            // Tabs
            "tab_list" => Ok(Self::TabList),
            "tab_new" => Ok(Self::TabNew),
            "tab_close" => Ok(Self::TabClose),
            "tab_select" => Ok(Self::TabSelect),

            // Dialog
            "dialog_accept" => Ok(Self::DialogAccept),
            "dialog_dismiss" => Ok(Self::DialogDismiss),

            // Storage & Network
            "cookies" => Ok(Self::Cookies),
            "cookies_set" => Ok(Self::CookiesSet),
            "storage_get" => Ok(Self::StorageGet),
            "storage_set" => Ok(Self::StorageSet),
            "network_requests" => Ok(Self::NetworkRequests),

            // Settings
            "set_viewport" => Ok(Self::SetViewport),
            "set_device" => Ok(Self::SetDevice),
            "set_geo" => Ok(Self::SetGeo),

            // Eval
            "eval" => Ok(Self::Eval),

            _ => Err(AgentError::ToolError(format!(
                "Unknown browser action: {}. Valid actions: open, close, snapshot, click, fill, screenshot, wait, wait_for_text, wait_for_url, wait_for_load, wait_for_download, wait_for_fn, wait_for_state, eval, get, get_attr, get_count, get_box, get_styles, back, forward, reload, type, press, hover, select, dblclick, focus, check, uncheck, scrollintoview, drag, upload, pdf, find, scroll, is, download, tab_list, tab_new, tab_close, tab_select, dialog_accept, dialog_dismiss, cookies, cookies_set, storage_get, storage_set, network_requests, set_viewport, set_device, set_geo",
                s
            ))),
        }
    }
}

impl BrowserAction {
    /// Convert the enum variant back to its string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            // Navigation
            Self::Open => "open",
            Self::Close => "close",
            Self::Snapshot => "snapshot",
            Self::Screenshot => "screenshot",
            Self::Back => "back",
            Self::Forward => "forward",
            Self::Reload => "reload",

            // Interaction
            Self::Click => "click",
            Self::Fill => "fill",
            Self::Type => "type",
            Self::Press => "press",
            Self::Hover => "hover",
            Self::Select => "select",
            Self::Dblclick => "dblclick",
            Self::Focus => "focus",
            Self::Check => "check",
            Self::Uncheck => "uncheck",
            Self::Scrollintoview => "scrollintoview",
            Self::Drag => "drag",
            Self::Upload => "upload",
            Self::Pdf => "pdf",

            // Query
            Self::Get => "get",
            Self::GetAttr => "get_attr",
            Self::GetCount => "get_count",
            Self::GetBox => "get_box",
            Self::GetStyles => "get_styles",

            // Wait
            Self::Wait => "wait",
            Self::WaitForText => "wait_for_text",
            Self::WaitForUrl => "wait_for_url",
            Self::WaitForLoad => "wait_for_load",
            Self::WaitForDownload => "wait_for_download",
            Self::WaitForFn => "wait_for_fn",
            Self::WaitForState => "wait_for_state",

            // State
            Self::Find => "find",
            Self::Scroll => "scroll",
            Self::Is => "is",
            Self::Download => "download",

            // Tabs
            Self::TabList => "tab_list",
            Self::TabNew => "tab_new",
            Self::TabClose => "tab_close",
            Self::TabSelect => "tab_select",

            // Dialog
            Self::DialogAccept => "dialog_accept",
            Self::DialogDismiss => "dialog_dismiss",

            // Storage & Network
            Self::Cookies => "cookies",
            Self::CookiesSet => "cookies_set",
            Self::StorageGet => "storage_get",
            Self::StorageSet => "storage_set",
            Self::NetworkRequests => "network_requests",

            // Settings
            Self::SetViewport => "set_viewport",
            Self::SetDevice => "set_device",
            Self::SetGeo => "set_geo",

            // Eval
            Self::Eval => "eval",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str_valid_actions() {
        let test_cases = vec![
            ("open", BrowserAction::Open),
            ("close", BrowserAction::Close),
            ("snapshot", BrowserAction::Snapshot),
            ("screenshot", BrowserAction::Screenshot),
            ("back", BrowserAction::Back),
            ("forward", BrowserAction::Forward),
            ("reload", BrowserAction::Reload),
            ("click", BrowserAction::Click),
            ("fill", BrowserAction::Fill),
            ("type", BrowserAction::Type),
            ("press", BrowserAction::Press),
            ("hover", BrowserAction::Hover),
            ("select", BrowserAction::Select),
            ("dblclick", BrowserAction::Dblclick),
            ("focus", BrowserAction::Focus),
            ("check", BrowserAction::Check),
            ("uncheck", BrowserAction::Uncheck),
            ("scrollintoview", BrowserAction::Scrollintoview),
            ("drag", BrowserAction::Drag),
            ("upload", BrowserAction::Upload),
            ("pdf", BrowserAction::Pdf),
            ("get", BrowserAction::Get),
            ("get_attr", BrowserAction::GetAttr),
            ("get_count", BrowserAction::GetCount),
            ("get_box", BrowserAction::GetBox),
            ("get_styles", BrowserAction::GetStyles),
            ("wait", BrowserAction::Wait),
            ("wait_for_text", BrowserAction::WaitForText),
            ("wait_for_url", BrowserAction::WaitForUrl),
            ("wait_for_load", BrowserAction::WaitForLoad),
            ("wait_for_download", BrowserAction::WaitForDownload),
            ("wait_for_fn", BrowserAction::WaitForFn),
            ("wait_for_state", BrowserAction::WaitForState),
            ("find", BrowserAction::Find),
            ("scroll", BrowserAction::Scroll),
            ("is", BrowserAction::Is),
            ("download", BrowserAction::Download),
            ("tab_list", BrowserAction::TabList),
            ("tab_new", BrowserAction::TabNew),
            ("tab_close", BrowserAction::TabClose),
            ("tab_select", BrowserAction::TabSelect),
            ("dialog_accept", BrowserAction::DialogAccept),
            ("dialog_dismiss", BrowserAction::DialogDismiss),
            ("cookies", BrowserAction::Cookies),
            ("cookies_set", BrowserAction::CookiesSet),
            ("storage_get", BrowserAction::StorageGet),
            ("storage_set", BrowserAction::StorageSet),
            ("network_requests", BrowserAction::NetworkRequests),
            ("set_viewport", BrowserAction::SetViewport),
            ("set_device", BrowserAction::SetDevice),
            ("set_geo", BrowserAction::SetGeo),
            ("eval", BrowserAction::Eval),
        ];

        for (s, expected) in test_cases {
            let result = BrowserAction::from_str(s);
            assert!(result.is_ok(), "Failed to parse '{}'", s);
            assert_eq!(result.unwrap(), expected);
        }
    }

    #[test]
    fn test_as_str_roundtrip() {
        let actions = vec![
            BrowserAction::Open,
            BrowserAction::Click,
            BrowserAction::Wait,
            BrowserAction::Eval,
            BrowserAction::Get,
            BrowserAction::TabList,
            BrowserAction::Cookies,
            BrowserAction::SetViewport,
            BrowserAction::DialogAccept,
            BrowserAction::NetworkRequests,
        ];

        for action in actions {
            let s = action.as_str();
            let parsed = BrowserAction::from_str(s).unwrap();
            assert_eq!(action, parsed, "Roundtrip failed for {:?}", action);
        }
    }

    #[test]
    fn test_all_actions_have_string_representation() {
        let test_actions = vec![
            BrowserAction::Open,
            BrowserAction::Close,
            BrowserAction::Snapshot,
            BrowserAction::Screenshot,
            BrowserAction::Back,
            BrowserAction::Forward,
            BrowserAction::Reload,
            BrowserAction::Click,
            BrowserAction::Fill,
            BrowserAction::Type,
            BrowserAction::Press,
            BrowserAction::Hover,
            BrowserAction::Select,
            BrowserAction::Dblclick,
            BrowserAction::Focus,
            BrowserAction::Check,
            BrowserAction::Uncheck,
            BrowserAction::Scrollintoview,
            BrowserAction::Drag,
            BrowserAction::Upload,
            BrowserAction::Pdf,
            BrowserAction::Get,
            BrowserAction::GetAttr,
            BrowserAction::GetCount,
            BrowserAction::GetBox,
            BrowserAction::GetStyles,
            BrowserAction::Wait,
            BrowserAction::WaitForText,
            BrowserAction::WaitForUrl,
            BrowserAction::WaitForLoad,
            BrowserAction::WaitForDownload,
            BrowserAction::WaitForFn,
            BrowserAction::WaitForState,
            BrowserAction::Find,
            BrowserAction::Scroll,
            BrowserAction::Is,
            BrowserAction::Download,
            BrowserAction::TabList,
            BrowserAction::TabNew,
            BrowserAction::TabClose,
            BrowserAction::TabSelect,
            BrowserAction::DialogAccept,
            BrowserAction::DialogDismiss,
            BrowserAction::Cookies,
            BrowserAction::CookiesSet,
            BrowserAction::StorageGet,
            BrowserAction::StorageSet,
            BrowserAction::NetworkRequests,
            BrowserAction::SetViewport,
            BrowserAction::SetDevice,
            BrowserAction::SetGeo,
            BrowserAction::Eval,
        ];

        for action in test_actions {
            let s = action.as_str();
            assert!(!s.is_empty(), "Empty string for {:?}", action);
            assert!(s.is_ascii(), "Non-ASCII string for {:?}: {}", action, s);
        }
    }

    #[test]
    fn test_copy_traits() {
        let action = BrowserAction::Click;
        let _copy = action;
        let _clone = action;
    }

    #[test]
    fn test_hash_equality() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(BrowserAction::Click);
        set.insert(BrowserAction::Open);
        set.insert(BrowserAction::Wait);

        assert_eq!(set.len(), 3);
        assert!(set.contains(&BrowserAction::Click));
        assert!(set.contains(&BrowserAction::Open));
        assert!(set.contains(&BrowserAction::Wait));
        assert!(!set.contains(&BrowserAction::Close));
    }
}
