# VoiceFlow Speech Input Method

VoiceFlow is a Windows speech input app. Use a shortcut to dictate into your
current app, or select text and describe how you want to rewrite it.

**Windows x64 · Early preview**

## Features

- Local speech recognition with SenseVoice.
- Optional AI cleanup, rewriting and translation.
- Support for Alibaba Cloud Bailian, Volcengine Ark, Tencent Hunyuan and
  custom OpenAI-compatible providers.
- Push-to-talk and toggle recording, with a floating status indicator.
- A desktop settings window, system tray, local history and Chinese/English UI.

## Getting started

This is a source preview. Follow the [build guide](docs/build-from-source.md)
to set up Python and the speech model, build the app and create a local desktop
package. Windows requires the WebView2 Evergreen Runtime.

Once set up:

1. Open **VoiceFlow.exe** and choose your recording shortcut in Settings.
2. Configure a provider and your own API key if you want AI-assisted editing.
3. Focus a text field, use the shortcut to record, and finish recording to
   insert the result.

See [desktop usage](docs/desktop-usage.md) for more details.

## Privacy

Speech recognition runs locally. AI refinement sends text to your configured
provider; selected-text editing can also send the selected text. History is
stored on your device, and API keys saved through Settings use Windows
Credential Manager.

## Preview status

Compatibility with different Windows apps is still being tested. Text insertion
and selection support depend on the target app. There is no always-on wake-word
listening.

Bug reports are welcome: include your Windows version, target app and steps to
reproduce the issue, without private recordings or API keys.

## Development and license

See the [build guide](docs/build-from-source.md) and
[developer reference](docs/developer-reference.md) for setup, configuration and
diagnostics.

VoiceFlow source is [MIT licensed](LICENSE). SenseVoice model weights and other
dependencies have [separate licenses](THIRD_PARTY_NOTICES.md).
