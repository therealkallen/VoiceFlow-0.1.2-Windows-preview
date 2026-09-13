# VoiceFlow

Windows x64 speech input — early preview.

## Start

1. Extract the entire ZIP and open **VoiceFlow.exe**.
2. Choose your recording shortcut in Settings.
3. Focus a text field, record with the shortcut, then finish recording to insert text.

Python and the SenseVoice model are included. No separate model download or
Python installation is needed. Microsoft Edge WebView2 Evergreen Runtime is
required; if missing, install it from https://developer.microsoft.com/microsoft-edge/webview2/.

Configure your own provider and API key to use AI cleanup, rewriting or
translation. Speech recognition is local; AI features send text (including
selected text when editing) to the provider you choose.

Closing the window keeps VoiceFlow in the system tray. Use the tray's Quit
action to exit. Finish recording before quitting. Don't run another VoiceFlow
copy at the same time.

This preview is unsigned. Text insertion and selected-text support vary by app;
compatibility on company-managed computers has not been verified.

## 项目与许可 / Project and licenses

项目：https://github.com/therealkallen/VoiceFlow-Speech-Input-Method

解压后打开 VoiceFlow.exe 即可，无需另装 Python 或下载模型。需要系统已安装 WebView2。
关闭窗口后程序仍在托盘运行；退出请使用托盘菜单。AI 功能需要你自己的服务商 API key。

VoiceFlow source: MIT, see LICENSE. SenseVoice / FunASR model and dependencies
have separate terms in THIRD_PARTY_NOTICES.md and licenses/. Keep these files
with the app. Use and redistribution of the bundled components are subject to
their respective terms, including the Microsoft runtime conditions in
licenses/runtime/python/LICENSE.txt.
