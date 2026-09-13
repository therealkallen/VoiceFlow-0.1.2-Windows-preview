use serde::{Deserialize, Serialize};
use shared_protocol::{
    RuntimeSettings, SelectedTextExecutionAction, SessionSummary, ShortcutMode,
    WakePhraseIntentAction, shortcut_to_string,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FailureGuidance {
    pub mode: String,
    pub guidance: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CommitPathSummary {
    pub outlook: String,
    pub signal: String,
    pub guidance: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VerificationPlan {
    pub focus: String,
    pub guidance: String,
    pub title: String,
    pub scenario: String,
    pub command: String,
    pub gesture: String,
    pub example: String,
    pub note: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ActionMixEntry {
    pub label: String,
    pub count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WorkloadFocusSummary {
    pub focus: String,
    pub guidance: String,
}

pub(crate) fn derive_failure_guidance(
    recording_failures: u64,
    recognizing_failures: u64,
    executing_failures: u64,
    committing_failures: u64,
    silence_gate_failures: u64,
    no_speech_failures: u64,
) -> Option<FailureGuidance> {
    let candidates = [
        (
            "Silence gate",
            silence_gate_failures,
            "Most failures are being rejected before ASR. The current silence gate may be too strict for the capture environment.",
        ),
        (
            "No speech",
            no_speech_failures,
            "Audio is reaching recognition, but the recognizer is still returning no usable speech. Check capture timing, microphone level, and phrasing.",
        ),
        (
            "Commit",
            committing_failures,
            "Speech is making it through recognition, but the final insertion path is failing in the target app. This points to app-compatibility or commit-transport issues.",
        ),
        (
            "Recording",
            recording_failures,
            "The runtime is failing before recognition begins. This points to capture start/stop or silence classification issues.",
        ),
        (
            "Recognizing",
            recognizing_failures,
            "The host is reaching the recognition stage and then failing. This points to the ASR or refinement path rather than hotkeys or insertion.",
        ),
        (
            "Executing",
            executing_failures,
            "Recognition is succeeding, but the edit or wake-phrase execution layer is failing before commit.",
        ),
    ];

    let (label, count, guidance) = candidates
        .into_iter()
        .filter(|(_, count, _)| *count > 0)
        .max_by_key(|(_, count, _)| *count)?;

    Some(FailureGuidance {
        mode: format!("{label} ({count})"),
        guidance: guidance.to_string(),
    })
}

pub(crate) fn derive_commit_path_summary(
    direct_unicode_commits: u64,
    clipboard_fallback_commits: u64,
    selection_replace_commits: u64,
    committing_failures: u64,
) -> Option<CommitPathSummary> {
    if committing_failures > 0 {
        return Some(CommitPathSummary {
            outlook: "At risk".to_string(),
            signal: format!("Commit reliability at risk ({committing_failures})"),
            guidance: "Late failures are happening during commit. The current target app or transport path still needs attention before commits can be treated as stable.".to_string(),
        });
    }

    if clipboard_fallback_commits > direct_unicode_commits && clipboard_fallback_commits > 0 {
        return Some(CommitPathSummary {
            outlook: "Fallback-heavy".to_string(),
            signal: format!("Clipboard fallback dominant ({clipboard_fallback_commits})"),
            guidance: "Successful commits are leaning on the clipboard-backed fallback more than direct Unicode input. Cross-app insertion is working, but not yet in the cleanest path.".to_string(),
        });
    }

    if direct_unicode_commits > 0 {
        return Some(CommitPathSummary {
            outlook: "Healthy".to_string(),
            signal: format!("Direct Unicode stable ({direct_unicode_commits})"),
            guidance: "Recent commits are mostly flowing through direct Unicode SendInput without needing the clipboard fallback, which is the healthier caret-insertion path.".to_string(),
        });
    }

    if selection_replace_commits > 0 {
        return Some(CommitPathSummary {
            outlook: "Edit-heavy".to_string(),
            signal: format!("Selection replace active ({selection_replace_commits})"),
            guidance: "The mirrored runtime is succeeding primarily through the selected-text replacement path. That is expected for edit-heavy sessions, but it does not tell us much about plain caret dictation targets.".to_string(),
        });
    }

    None
}

pub(crate) fn derive_verification_plan(
    dominant_failure_mode: Option<&str>,
    commit_path_outlook: Option<&str>,
    workload_focus: Option<&str>,
    settings: &RuntimeSettings,
) -> Option<VerificationPlan> {
    let (focus, guidance, title, scenario, example) = if matches!(
        commit_path_outlook,
        Some("At risk")
    ) || matches!(dominant_failure_mode, Some(mode) if mode.starts_with("Commit"))
    {
        (
            "Insertion reliability",
            "The next check should happen in the real target app. Confirm that text lands once, in the right place, and without new late commit failures.",
            "Verify the next real-app insertion check.",
            "Use the real target app and confirm that the resulting text lands once, in the correct place, without a late commit failure or duplicated insertion.",
            "Example: say a short sentence like \"all right, testing the insertion path again\" and confirm it lands once in the target app.",
        )
    } else if matches!(commit_path_outlook, Some("Fallback-heavy")) {
        (
            "Caret coverage",
            "The next check should verify whether your intended app still leans on clipboard fallback or can commit through the cleaner direct caret path.",
            "Verify the next caret-insertion check.",
            "Use a plain caret dictation case in the app you care about, and check whether the text lands through the cleaner caret path instead of relying on clipboard-style fallback behavior.",
            "Example: place the caret in the target app, say \"this should land through the cleaner caret path,\" and confirm the text lands once in place.",
        )
    } else if matches!(dominant_failure_mode, Some(mode) if mode.starts_with("Silence gate") || mode.starts_with("No speech") || mode.starts_with("Recording") || mode.starts_with("Recognizing"))
    {
        (
            "Speech capture",
            "The next check should focus on capture timing, microphone level, and speech clarity before worrying about insertion behavior.",
            "Verify the next speech-capture check.",
            "Use a plain dictation take with no text selected, and focus on whether the runtime cleanly captures and recognizes a short sentence before you judge insertion.",
            "Example: say \"all right, testing the speech capture path\" with no text selected and ignore insertion quality until recognition looks stable.",
        )
    } else if matches!(dominant_failure_mode, Some(mode) if mode.starts_with("Executing"))
        || matches!(commit_path_outlook, Some("Edit-heavy"))
    {
        (
            "Edit execution",
            "The next check should use a selected-text edit case so you can confirm the instruction is interpreted correctly before it reaches commit.",
            "Verify the next selected-text edit check.",
            "Highlight a short phrase first, use an explicit edit instruction like uppercase, and confirm that the runtime routes into selected-text edit before it commits.",
            "Example: highlight a short phrase and say \"uppercase\" before checking whether the selection is replaced correctly.",
        )
    } else if matches!(commit_path_outlook, Some("Healthy")) {
        match workload_focus {
            Some("Formatting edits") => (
                "Real-app confidence",
                "The runtime looks healthy overall. The next check should confirm that the same formatting-style edit path still holds in the real app you care about.",
                "Verify the next real-app formatting check.",
                "Highlight a short phrase in the real target app, use a formatting instruction like uppercase or title case, and confirm that the edit path still routes and lands cleanly outside the prototype flow.",
                "Example: highlight a short phrase and say \"title case\" to confirm the formatting edit still lands cleanly in the real app.",
            ),
            Some("Structural edits") => (
                "Real-app confidence",
                "The runtime looks healthy overall. The next check should confirm that a structure-oriented selected-text edit still holds in the real app you care about.",
                "Verify the next real-app structural edit check.",
                "Highlight a short multi-line block in the real target app, use a structural instruction like numbered list or clean up spacing, and confirm that the edit path still lands cleanly.",
                "Example: highlight a short multi-line block and say \"numbered list\" to confirm the structural edit still lands cleanly in the real app.",
            ),
            Some("Drafting workload") => (
                "Real-app confidence",
                "The runtime looks healthy overall. The next check should confirm that drafting-style wake-phrase requests still expand cleanly in the app you care about.",
                "Verify the next real-app drafting check.",
                "Run a wake-phrase drafting request such as a short email or reply in the real target app, and confirm that the drafted output still lands cleanly outside the prototype path.",
                "Example: say \"Hey VoiceFlow, draft an email to finance about the revised budget\" and confirm the draft lands cleanly in the real app.",
            ),
            Some("Synthesis workload") => (
                "Real-app confidence",
                "The runtime looks healthy overall. The next check should confirm that synthesis-style wake-phrase requests still expand cleanly in the app you care about.",
                "Verify the next real-app synthesis check.",
                "Run a wake-phrase summary, checklist, or plan request in the real target app, and confirm that the structured output still lands cleanly outside the prototype path.",
                "Example: say \"Hey VoiceFlow, summarize the launch update\" and confirm the structured output lands cleanly in the real app.",
            ),
            Some("Edit-heavy workload") => (
                "Real-app confidence",
                "The runtime looks healthy overall. The next check should confirm that selected-text edit behavior is still clean in the real app you care about.",
                "Verify the next real-app edit check.",
                "Highlight a short phrase in the real target app, use an explicit selected-text instruction, and confirm that the edit path still routes and lands cleanly outside the prototype flow.",
                "Example: highlight a short phrase and say \"uppercase\" to confirm the edit path still lands cleanly in the real app.",
            ),
            Some("Intent-heavy workload") => (
                "Real-app confidence",
                "The runtime looks healthy overall. The next check should confirm that wake-phrase intent behavior is still clean in the real app you care about.",
                "Verify the next real-app intent check.",
                "Run a short wake-phrase intent request in the real target app, and confirm that the generated output still lands cleanly outside the prototype flow.",
                "Example: say \"Hey VoiceFlow, draft a short reply to the client\" and confirm the generated output lands cleanly in the real app.",
            ),
            _ => (
                "Real-app confidence",
                "The runtime looks healthy overall. The next check should confirm that the same behavior still holds in the app you actually care about.",
                "Verify the next real-app confidence check.",
                "Repeat a normal real-app dictation or edit case in the app you actually care about, so the next mirrored report confirms the runtime is still healthy outside the prototype path.",
                "Example: run one normal dictation or edit case in the real target app and confirm the next mirrored report stays healthy.",
            ),
        }
    } else {
        return None;
    };

    let gesture = match settings.shortcut_mode {
        ShortcutMode::PushToTalk => {
            format!(
                "Hold {} while speaking, then release to stop.",
                shortcut_to_string(&settings.dictation_shortcut)
            )
        }
        ShortcutMode::Toggle => {
            format!(
                "Press {} once to start, then press it again to stop.",
                shortcut_to_string(&settings.dictation_shortcut)
            )
        }
    };
    let note = format!(
        "{guidance} {scenario} {example} For the next check, {gesture} If text is selected, the runtime should route into edit mode instead of plain dictation."
    );

    Some(VerificationPlan {
        focus: focus.to_string(),
        guidance: guidance.to_string(),
        title: title.to_string(),
        scenario: scenario.to_string(),
        command: "cargo run -p input-host -- --serve-live".to_string(),
        gesture,
        example: example.to_string(),
        note,
    })
}

pub(crate) fn derive_workload_focus(
    dictation_sessions: u64,
    selected_text_sessions: u64,
    wake_phrase_sessions: u64,
    selected_text_action_mix: &[ActionMixEntry],
    wake_phrase_action_mix: &[ActionMixEntry],
) -> Option<WorkloadFocusSummary> {
    let largest = dictation_sessions
        .max(selected_text_sessions)
        .max(wake_phrase_sessions);
    if largest == 0 {
        return None;
    }

    if selected_text_sessions == largest && selected_text_sessions > 0 {
        let top_label = selected_text_action_mix
            .first()
            .map(|entry| entry.label.as_str());
        let (focus, guidance) = match top_label {
            Some(
                "Uppercase"
                | "Lowercase"
                | "Title case"
                | "Sentence case"
                | "Snake case"
                | "Kebab case"
                | "Camel case"
                | "Pascal case"
                | "Constant case"
                | "Inline code"
                | "Code block"
                | "Strip code fence"
                | "Markdown bold"
                | "Markdown italic"
                | "Strip markdown emphasis"
                | "Wrap in quotes"
                | "Checklist"
                | "Quote block",
            ) => (
                "Formatting edits",
                "Recent successful edit traffic is dominated by formatting-style transforms, so the runtime is being used more like a fast text-shaping tool than a drafting surface.",
            ),
            Some(
                "Bullet list"
                | "Numbered list"
                | "Sort lines"
                | "Deduplicate lines"
                | "Remove empty lines"
                | "Comma-separated"
                | "Pipe-separated"
                | "Tab-separated"
                | "Semicolon-separated"
                | "JSON array"
                | "Quoted CSV"
                | "SQL IN list"
                | "YAML list"
                | "YAML mapping"
                | "Markdown table"
                | "Header block"
                | "JSON object"
                | "Env block"
                | "Query string"
                | "TOML table"
                | "Shell exports"
                | "PowerShell env"
                | "curl headers"
                | "Python dict"
                | "JavaScript object"
                | "Ruby hash"
                | "SQL VALUES rows"
                | "Strip list markers"
                | "Sentence per line"
                | "Markdown heading"
                | "Single paragraph"
                | "Spacing cleanup"
                | "Prompt scaffold",
            ) => (
                "Structural edits",
                "Recent successful edit traffic is dominated by structure-oriented transforms, so the runtime is being used heavily for cleanup, reshaping, and prompt-style reformatting.",
            ),
            _ => (
                "Edit-heavy workload",
                "Selected-text edit sessions currently dominate the mirrored runtime history, so manual edit execution is the main workload rather than plain dictation or intent drafting.",
            ),
        };
        return Some(WorkloadFocusSummary {
            focus: focus.to_string(),
            guidance: guidance.to_string(),
        });
    }

    if wake_phrase_sessions == largest && wake_phrase_sessions > 0 {
        let top_label = wake_phrase_action_mix
            .first()
            .map(|entry| entry.label.as_str());
        let (focus, guidance) = match top_label {
            Some("Draft email" | "Reply draft" | "General draft") => (
                "Drafting workload",
                "Recent successful wake-phrase traffic is dominated by drafting-style actions, so the runtime is acting more like a quick composition surface than a pure dictation tool.",
            ),
            Some("Summarize" | "Checklist" | "Bullet plan" | "Rewrite") => (
                "Synthesis workload",
                "Recent successful wake-phrase traffic is dominated by summarizing, planning, or rewriting actions, so the runtime is being used more for structured synthesis than freeform drafting.",
            ),
            _ => (
                "Intent-heavy workload",
                "Wake-phrase intent sessions currently dominate the mirrored runtime history, so the runtime is being used more for intent execution than plain dictation or selected-text editing.",
            ),
        };
        return Some(WorkloadFocusSummary {
            focus: focus.to_string(),
            guidance: guidance.to_string(),
        });
    }

    Some(WorkloadFocusSummary {
        focus: "Dictation-heavy workload".to_string(),
        guidance: "Plain dictation currently dominates the mirrored runtime history, so the core speech-capture and insertion path remains the primary usage pattern.".to_string(),
    })
}

pub(crate) fn derive_selected_text_action_mix(history: &[SessionSummary]) -> Vec<ActionMixEntry> {
    let mut counts = BTreeMap::new();
    for summary in history {
        let Some(execution) = &summary.selected_text_execution else {
            continue;
        };
        let label = selected_text_action_label(&execution.action).to_string();
        *counts.entry(label).or_insert(0_u64) += 1;
    }

    summarize_action_mix(counts)
}

pub(crate) fn derive_wake_phrase_action_mix(history: &[SessionSummary]) -> Vec<ActionMixEntry> {
    let mut counts = BTreeMap::new();
    for summary in history {
        let Some(execution) = &summary.wake_phrase_execution else {
            continue;
        };
        let label = wake_phrase_action_label(&execution.action).to_string();
        *counts.entry(label).or_insert(0_u64) += 1;
    }

    summarize_action_mix(counts)
}

pub(crate) fn selected_text_action_label(action: &SelectedTextExecutionAction) -> &'static str {
    match action {
        SelectedTextExecutionAction::Uppercase => "Uppercase",
        SelectedTextExecutionAction::Lowercase => "Lowercase",
        SelectedTextExecutionAction::TitleCase => "Title case",
        SelectedTextExecutionAction::SentenceCase => "Sentence case",
        SelectedTextExecutionAction::SnakeCase => "Snake case",
        SelectedTextExecutionAction::KebabCase => "Kebab case",
        SelectedTextExecutionAction::CamelCase => "Camel case",
        SelectedTextExecutionAction::PascalCase => "Pascal case",
        SelectedTextExecutionAction::ConstantCase => "Constant case",
        SelectedTextExecutionAction::InlineCode => "Inline code",
        SelectedTextExecutionAction::CodeBlock => "Code block",
        SelectedTextExecutionAction::StripCodeFence => "Strip code fence",
        SelectedTextExecutionAction::MarkdownBold => "Markdown bold",
        SelectedTextExecutionAction::MarkdownItalic => "Markdown italic",
        SelectedTextExecutionAction::StripMarkdownEmphasis => "Strip markdown emphasis",
        SelectedTextExecutionAction::WrapInQuotes => "Wrap in quotes",
        SelectedTextExecutionAction::BulletList => "Bullet list",
        SelectedTextExecutionAction::Checklist => "Checklist",
        SelectedTextExecutionAction::QuoteBlock => "Quote block",
        SelectedTextExecutionAction::NumberedList => "Numbered list",
        SelectedTextExecutionAction::SortLines => "Sort lines",
        SelectedTextExecutionAction::DeduplicateLines => "Deduplicate lines",
        SelectedTextExecutionAction::RemoveEmptyLines => "Remove empty lines",
        SelectedTextExecutionAction::CommaSeparated => "Comma-separated",
        SelectedTextExecutionAction::PipeSeparated => "Pipe-separated",
        SelectedTextExecutionAction::TabSeparated => "Tab-separated",
        SelectedTextExecutionAction::SemicolonSeparated => "Semicolon-separated",
        SelectedTextExecutionAction::JsonArray => "JSON array",
        SelectedTextExecutionAction::QuotedCsv => "Quoted CSV",
        SelectedTextExecutionAction::SqlInList => "SQL IN list",
        SelectedTextExecutionAction::YamlList => "YAML list",
        SelectedTextExecutionAction::YamlMapping => "YAML mapping",
        SelectedTextExecutionAction::MarkdownTable => "Markdown table",
        SelectedTextExecutionAction::HeaderBlock => "Header block",
        SelectedTextExecutionAction::JsonObject => "JSON object",
        SelectedTextExecutionAction::EnvBlock => "Env block",
        SelectedTextExecutionAction::QueryString => "Query string",
        SelectedTextExecutionAction::TomlTable => "TOML table",
        SelectedTextExecutionAction::ShellExports => "Shell exports",
        SelectedTextExecutionAction::PowershellEnv => "PowerShell env",
        SelectedTextExecutionAction::CurlHeaders => "curl headers",
        SelectedTextExecutionAction::PythonDict => "Python dict",
        SelectedTextExecutionAction::JavascriptObject => "JavaScript object",
        SelectedTextExecutionAction::RubyHash => "Ruby hash",
        SelectedTextExecutionAction::SqlValuesRows => "SQL VALUES rows",
        SelectedTextExecutionAction::StripListMarkers => "Strip list markers",
        SelectedTextExecutionAction::SentencePerLine => "Sentence per line",
        SelectedTextExecutionAction::MarkdownHeading => "Markdown heading",
        SelectedTextExecutionAction::SingleParagraph => "Single paragraph",
        SelectedTextExecutionAction::CleanupSpacing => "Spacing cleanup",
        SelectedTextExecutionAction::PolishWriting => "Polish writing",
        SelectedTextExecutionAction::ConciseRewrite => "Concise rewrite",
        SelectedTextExecutionAction::FormalRewrite => "Formal rewrite",
        SelectedTextExecutionAction::BulletSummary => "Bullet summary",
        SelectedTextExecutionAction::GeneralProviderEdit => "General edit",
        SelectedTextExecutionAction::PromptScaffold => "Prompt scaffold",
    }
}

pub(crate) fn wake_phrase_action_label(action: &WakePhraseIntentAction) -> &'static str {
    match action {
        WakePhraseIntentAction::DraftEmail => "Draft email",
        WakePhraseIntentAction::Summarize => "Summarize",
        WakePhraseIntentAction::BulletPlan => "Bullet plan",
        WakePhraseIntentAction::Rewrite => "Rewrite",
        WakePhraseIntentAction::Checklist => "Checklist",
        WakePhraseIntentAction::ReplyMessage => "Reply draft",
        WakePhraseIntentAction::GeneralDraft => "General draft",
    }
}

fn summarize_action_mix(counts: BTreeMap<String, u64>) -> Vec<ActionMixEntry> {
    let mut items = counts
        .into_iter()
        .map(|(label, count)| ActionMixEntry { label, count })
        .collect::<Vec<_>>();
    items.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.label.cmp(&right.label))
    });
    items.truncate(3);
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared_protocol::{
        AsrProvider, CommitStatus, CommitTransport, RouteDecision, RouteName,
        SelectedTextExecutionSummary, SessionKind, SessionState, WakePhraseIntentExecutionSummary,
    };

    fn build_summary() -> SessionSummary {
        SessionSummary {
            session_id: 1,
            session_kind: SessionKind::Dictation,
            final_state: SessionState::Committed,
            completed_at_epoch_ms: None,
            start_feedback_latency_ms: Some(0),
            recording_start_latency_ms: Some(250),
            total_session_latency_ms: 1_000,
            audio_duration_ms: 800,
            audio_peak_level: 0.25,
            audio_rms_level: 0.01,
            asr_diagnostics: None,
            refine_diagnostics: None,
            recognized_text: "hello".to_string(),
            committed_text: "Hello.".to_string(),
            committed_text_count: None,
            route_decision: RouteDecision {
                route_name: RouteName::LocalAsrOnly,
                asr_provider: AsrProvider::Local,
                refine_provider: None,
                refinement_applied: false,
                reason: "test".to_string(),
                refine_fast_path_used: false,
                refine_fast_path_reason: "not_evaluated".to_string(),
                cloud_refine_skipped: false,
                dictation_routing: None,
            },
            degraded_to_asr: false,
            fallback_reason: None,
            commit_status: CommitStatus::Success,
            commit_transport: CommitTransport::DirectUnicodeSendInput,
            commit_failure_reason: None,
            mode_reason: "test".to_string(),
            selected_text_execution: None,
            wake_phrase_execution: None,
            instructed_dictation_execution: None,
        }
    }

    #[test]
    fn prefers_insertion_focus_when_commit_path_is_at_risk() {
        let settings = RuntimeSettings::default();

        let plan = derive_verification_plan(Some("Commit (2)"), Some("At risk"), None, &settings)
            .expect("plan should be present");

        assert_eq!(plan.focus, "Insertion reliability");
        assert!(plan.guidance.contains("real target app"));
    }

    #[test]
    fn derives_clipboard_fallback_commit_path_summary() {
        let summary = derive_commit_path_summary(1, 3, 0, 0).expect("summary should be present");

        assert_eq!(summary.outlook, "Fallback-heavy");
        assert_eq!(summary.signal, "Clipboard fallback dominant (3)");
        assert!(summary.guidance.contains("clipboard-backed fallback"));
    }

    #[test]
    fn derives_push_to_talk_gesture_from_runtime_settings() {
        let mut settings = RuntimeSettings::default();
        settings.shortcut_mode = ShortcutMode::PushToTalk;

        let plan = derive_verification_plan(Some("Commit (1)"), Some("At risk"), None, &settings)
            .expect("plan should be present");

        assert!(plan.gesture.starts_with("Hold "));
        assert!(plan.gesture.contains("while speaking"));
        assert!(plan.gesture.contains("release to stop"));
    }

    #[test]
    fn verification_plan_includes_host_owned_note() {
        let settings = RuntimeSettings::default();

        let plan = derive_verification_plan(Some("Commit (1)"), Some("At risk"), None, &settings)
            .expect("plan should be present");

        assert!(plan.note.contains(&plan.guidance));
        assert!(plan.note.contains(&plan.scenario));
        assert!(plan.note.contains(&plan.example));
        assert!(plan.note.contains(&plan.gesture));
        assert!(
            plan.note
                .contains("route into edit mode instead of plain dictation")
        );
    }

    #[test]
    fn derives_selected_text_action_mix_from_successful_history() {
        let mut uppercase = build_summary();
        uppercase.session_kind = SessionKind::SelectedTextEdit;
        uppercase.selected_text_execution = Some(SelectedTextExecutionSummary {
            action: SelectedTextExecutionAction::Uppercase,
            strategy: "uppercase".to_string(),
            provider_diagnostics: None,
        });

        let mut numbered = build_summary();
        numbered.session_kind = SessionKind::SelectedTextEdit;
        numbered.selected_text_execution = Some(SelectedTextExecutionSummary {
            action: SelectedTextExecutionAction::NumberedList,
            strategy: "numbered".to_string(),
            provider_diagnostics: None,
        });

        let mut uppercase_two = build_summary();
        uppercase_two.session_kind = SessionKind::SelectedTextEdit;
        uppercase_two.selected_text_execution = Some(SelectedTextExecutionSummary {
            action: SelectedTextExecutionAction::Uppercase,
            strategy: "uppercase".to_string(),
            provider_diagnostics: None,
        });

        let mix = derive_selected_text_action_mix(&[uppercase, numbered, uppercase_two]);

        assert_eq!(
            mix,
            vec![
                ActionMixEntry {
                    label: "Uppercase".to_string(),
                    count: 2
                },
                ActionMixEntry {
                    label: "Numbered list".to_string(),
                    count: 1
                }
            ]
        );
    }

    #[test]
    fn derives_wake_phrase_action_mix_from_successful_history() {
        let mut draft = build_summary();
        draft.session_kind = SessionKind::WakePhraseIntent;
        draft.wake_phrase_execution = Some(WakePhraseIntentExecutionSummary {
            action: WakePhraseIntentAction::DraftEmail,
            strategy: "draft".to_string(),
        });

        let mut summarize = build_summary();
        summarize.session_kind = SessionKind::WakePhraseIntent;
        summarize.wake_phrase_execution = Some(WakePhraseIntentExecutionSummary {
            action: WakePhraseIntentAction::Summarize,
            strategy: "summarize".to_string(),
        });

        let mix = derive_wake_phrase_action_mix(&[draft, summarize]);

        assert_eq!(
            mix,
            vec![
                ActionMixEntry {
                    label: "Draft email".to_string(),
                    count: 1
                },
                ActionMixEntry {
                    label: "Summarize".to_string(),
                    count: 1
                }
            ]
        );
    }

    #[test]
    fn derives_structural_edit_workload_focus_from_action_mix() {
        let summary = derive_workload_focus(
            1,
            4,
            1,
            &[ActionMixEntry {
                label: "Numbered list".to_string(),
                count: 3,
            }],
            &[],
        )
        .expect("workload focus should be present");

        assert_eq!(summary.focus, "Structural edits");
        assert!(summary.guidance.contains("structure-oriented transforms"));
    }

    #[test]
    fn derives_drafting_workload_focus_from_intent_mix() {
        let summary = derive_workload_focus(
            1,
            1,
            3,
            &[],
            &[ActionMixEntry {
                label: "Draft email".to_string(),
                count: 2,
            }],
        )
        .expect("workload focus should be present");

        assert_eq!(summary.focus, "Drafting workload");
        assert!(summary.guidance.contains("drafting-style actions"));
    }

    #[test]
    fn verification_plan_uses_workload_focus_for_healthy_runtime() {
        let settings = RuntimeSettings::default();

        let plan =
            derive_verification_plan(None, Some("Healthy"), Some("Structural edits"), &settings)
                .expect("plan should be present");

        assert_eq!(plan.focus, "Real-app confidence");
        assert_eq!(
            plan.title,
            "Verify the next real-app structural edit check."
        );
        assert!(plan.scenario.contains("numbered list"));
        assert!(plan.example.contains("numbered list"));
    }
}
