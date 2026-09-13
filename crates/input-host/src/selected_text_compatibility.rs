use serde::{Deserialize, Serialize};

const PREFIX: &str = "selected-text compatibility [";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SelectedTextFailureStage {
    ClipboardSnapshot,
    HotkeyRelease,
    SelectionCopy,
    SelectionRead,
    SelectionReplace,
    ClipboardRestore,
}

impl SelectedTextFailureStage {
    fn token(self) -> &'static str {
        match self {
            Self::ClipboardSnapshot => "ClipboardSnapshot",
            Self::HotkeyRelease => "HotkeyRelease",
            Self::SelectionCopy => "SelectionCopy",
            Self::SelectionRead => "SelectionRead",
            Self::SelectionReplace => "SelectionReplace",
            Self::ClipboardRestore => "ClipboardRestore",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SelectedTextFailureReason {
    NoForegroundWindow,
    ClipboardBusy,
    UnsupportedClipboardContents,
    HotkeyStillPressed,
    CopyTimedOut,
    ClipboardChangedWithoutText,
    EmptySelection,
    ClipboardReadFailed,
    CopyShortcutFailed,
    ClipboardWriteFailed,
    PasteShortcutFailed,
    ClipboardRestoreFailed,
}

impl SelectedTextFailureReason {
    fn token(self) -> &'static str {
        match self {
            Self::NoForegroundWindow => "NoForegroundWindow",
            Self::ClipboardBusy => "ClipboardBusy",
            Self::UnsupportedClipboardContents => "UnsupportedClipboardContents",
            Self::HotkeyStillPressed => "HotkeyStillPressed",
            Self::CopyTimedOut => "CopyTimedOut",
            Self::ClipboardChangedWithoutText => "ClipboardChangedWithoutText",
            Self::EmptySelection => "EmptySelection",
            Self::ClipboardReadFailed => "ClipboardReadFailed",
            Self::CopyShortcutFailed => "CopyShortcutFailed",
            Self::ClipboardWriteFailed => "ClipboardWriteFailed",
            Self::PasteShortcutFailed => "PasteShortcutFailed",
            Self::ClipboardRestoreFailed => "ClipboardRestoreFailed",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::NoForegroundWindow => "Foreground focus unavailable",
            Self::ClipboardBusy => "Clipboard open contention",
            Self::UnsupportedClipboardContents => "Clipboard snapshot unsupported",
            Self::HotkeyStillPressed => "Shortcut modifiers still pressed",
            Self::CopyTimedOut => "Ctrl+C capture timed out",
            Self::ClipboardChangedWithoutText => "Copy produced non-text clipboard data",
            Self::EmptySelection => "Captured selection was empty",
            Self::ClipboardReadFailed => "Clipboard text could not be read",
            Self::CopyShortcutFailed => "Ctrl+C gesture failed",
            Self::ClipboardWriteFailed => "Clipboard write failed",
            Self::PasteShortcutFailed => "Ctrl+V replacement failed",
            Self::ClipboardRestoreFailed => "Clipboard restore failed",
        }
    }

    fn guidance(self) -> &'static str {
        match self {
            Self::NoForegroundWindow => {
                "Keep the target app focused while the temporary selected-text probe runs."
            }
            Self::ClipboardBusy => {
                "Another app held the clipboard too long. Retry after clipboard-heavy tools settle."
            }
            Self::UnsupportedClipboardContents => {
                "This prototype can only snapshot empty, Unicode, or ANSI clipboard text before it sends Ctrl+C/Ctrl+V."
            }
            Self::HotkeyStillPressed => {
                "Release the shortcut cleanly before the temporary Ctrl+C selection probe starts."
            }
            Self::CopyTimedOut => {
                "The focused app did not expose selected text through synthetic Ctrl+C within the prototype timeout window."
            }
            Self::ClipboardChangedWithoutText => {
                "The focused app changed the clipboard, but not into plain text the prototype can read back safely."
            }
            Self::EmptySelection => {
                "The app returned empty or whitespace-only text, so the host could not trust that a real selection was captured."
            }
            Self::ClipboardReadFailed => {
                "The clipboard changed, but the temporary selected-text path could not decode the text payload."
            }
            Self::CopyShortcutFailed => {
                "The host could not send the temporary Ctrl+C gesture into the focused app reliably."
            }
            Self::ClipboardWriteFailed => {
                "The host could not stage replacement text onto the clipboard for the temporary selected-text flow."
            }
            Self::PasteShortcutFailed => {
                "The focused app did not accept the temporary Ctrl+V replacement gesture reliably."
            }
            Self::ClipboardRestoreFailed => {
                "The edit path ran, but clipboard restoration failed, so the session needs manual verification before reuse."
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SelectedTextCompatibilityIssue {
    pub stage: String,
    pub reason: String,
    pub guidance: String,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SelectedTextCompatibilitySummary {
    pub signal: String,
    pub guidance: String,
    pub stage: String,
    pub reason: String,
    pub count: u64,
}

pub(crate) fn format_selected_text_failure(
    stage: SelectedTextFailureStage,
    reason: SelectedTextFailureReason,
    detail: impl Into<String>,
) -> String {
    format!(
        "{PREFIX}{}/{}]: {}",
        stage.token(),
        reason.token(),
        detail.into()
    )
}

pub(crate) fn parse_selected_text_failure(message: &str) -> Option<SelectedTextCompatibilityIssue> {
    let remainder = message.strip_prefix(PREFIX)?;
    let (header, detail) = remainder.split_once("]: ")?;
    let (stage_token, reason_token) = header.split_once('/')?;
    let reason = reason_from_token(reason_token)?;

    Some(SelectedTextCompatibilityIssue {
        stage: stage_token.to_string(),
        reason: reason.label().to_string(),
        guidance: reason.guidance().to_string(),
        detail: detail.to_string(),
    })
}

pub(crate) fn selected_text_compatibility_signal(issue: &SelectedTextCompatibilityIssue) -> String {
    format!("{}: {}", issue.stage, issue.reason)
}

pub(crate) fn derive_selected_text_compatibility_summary<'a, I>(
    messages: I,
) -> Option<SelectedTextCompatibilitySummary>
where
    I: IntoIterator<Item = &'a str>,
{
    use std::collections::BTreeMap;

    let mut counts: BTreeMap<(String, String, String), u64> = BTreeMap::new();
    for issue in messages.into_iter().filter_map(parse_selected_text_failure) {
        *counts
            .entry((
                issue.stage.clone(),
                issue.reason.clone(),
                issue.guidance.clone(),
            ))
            .or_insert(0) += 1;
    }

    let ((stage, reason, guidance), count) = counts
        .into_iter()
        .max_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(&right.0)))?;

    Some(SelectedTextCompatibilitySummary {
        signal: format!("{reason} ({count})"),
        guidance,
        stage,
        reason,
        count,
    })
}

fn reason_from_token(token: &str) -> Option<SelectedTextFailureReason> {
    match token {
        "NoForegroundWindow" => Some(SelectedTextFailureReason::NoForegroundWindow),
        "ClipboardBusy" => Some(SelectedTextFailureReason::ClipboardBusy),
        "UnsupportedClipboardContents" => {
            Some(SelectedTextFailureReason::UnsupportedClipboardContents)
        }
        "HotkeyStillPressed" => Some(SelectedTextFailureReason::HotkeyStillPressed),
        "CopyTimedOut" => Some(SelectedTextFailureReason::CopyTimedOut),
        "ClipboardChangedWithoutText" => {
            Some(SelectedTextFailureReason::ClipboardChangedWithoutText)
        }
        "EmptySelection" => Some(SelectedTextFailureReason::EmptySelection),
        "ClipboardReadFailed" => Some(SelectedTextFailureReason::ClipboardReadFailed),
        "CopyShortcutFailed" => Some(SelectedTextFailureReason::CopyShortcutFailed),
        "ClipboardWriteFailed" => Some(SelectedTextFailureReason::ClipboardWriteFailed),
        "PasteShortcutFailed" => Some(SelectedTextFailureReason::PasteShortcutFailed),
        "ClipboardRestoreFailed" => Some(SelectedTextFailureReason::ClipboardRestoreFailed),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SelectedTextFailureReason, SelectedTextFailureStage,
        derive_selected_text_compatibility_summary, format_selected_text_failure,
        parse_selected_text_failure, selected_text_compatibility_signal,
    };

    #[test]
    fn parses_selected_text_failure_prefix() {
        let issue = parse_selected_text_failure(&format_selected_text_failure(
            SelectedTextFailureStage::SelectionCopy,
            SelectedTextFailureReason::CopyTimedOut,
            "capture attempt 2 of 2 timed out after 1500 ms",
        ))
        .expect("prefixed selected-text failure should parse");

        assert_eq!(issue.stage, "SelectionCopy");
        assert_eq!(issue.reason, "Ctrl+C capture timed out");
        assert!(issue.guidance.contains("synthetic Ctrl+C"));
    }

    #[test]
    fn derives_dominant_selected_text_compatibility_summary() {
        let summary = derive_selected_text_compatibility_summary([
            format_selected_text_failure(
                SelectedTextFailureStage::SelectionCopy,
                SelectedTextFailureReason::CopyTimedOut,
                "attempt 1",
            )
            .as_str(),
            format_selected_text_failure(
                SelectedTextFailureStage::SelectionCopy,
                SelectedTextFailureReason::CopyTimedOut,
                "attempt 2",
            )
            .as_str(),
            format_selected_text_failure(
                SelectedTextFailureStage::SelectionReplace,
                SelectedTextFailureReason::PasteShortcutFailed,
                "attempt 1",
            )
            .as_str(),
        ])
        .expect("dominant selected-text summary should derive");

        assert_eq!(summary.stage, "SelectionCopy");
        assert_eq!(summary.reason, "Ctrl+C capture timed out");
        assert_eq!(summary.count, 2);
    }

    #[test]
    fn formats_selected_text_compatibility_signal() {
        let issue = parse_selected_text_failure(&format_selected_text_failure(
            SelectedTextFailureStage::SelectionReplace,
            SelectedTextFailureReason::PasteShortcutFailed,
            "the app ignored Ctrl+V",
        ))
        .expect("compatibility failure should parse");

        assert_eq!(
            selected_text_compatibility_signal(&issue),
            "SelectionReplace: Ctrl+V replacement failed"
        );
    }
}
