# Overlay UI

This folder is the single production renderer for the live VoiceFlow overlay:

- `index.html` defines the compact status widget.
- `src/styles.css` defines its fixed-size presentation and state animations.
- `src/main.js` maps runtime session states to localized presentation states.

On Windows, `input-host` creates one transparent Wry/WebView2 window and serves
these assets through the private `voiceflow://overlay/` protocol. The host
injects state updates directly into the page. The native layer owns window
lifecycle and placement; there is no second native-drawn widget.

## Behavior

The widget is hidden while idle and appears at the bottom center of the active
monitor during voice input. Its presentation states are:

- Listening: animated audio bars and `Listening` / `正在说话`.
- Thinking: animated dots and `Thinking` / `思考中`.
- Done: a brief check and `Done` / `已完成`.
- Error: a compact warning with localized silence or generic failure copy.

The window is always on top, excluded from the taskbar, click-through, and
created with no-activate behavior so it does not take keyboard focus from the
target app. Placement uses the foreground monitor work area and effective DPI,
including monitors with negative coordinates. UI language is carried in each
runtime payload and updates after a Settings language change without restarting
the live host.

The generated development mirror is `runtime/overlay-state.js` beside the
settings file. The native protocol reads it from user data; static packaged
`src/runtime-state.js` is an empty placeholder. Live native updates remain the
production transport. Do not distribute private runtime snapshots.
