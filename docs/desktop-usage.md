# VoiceFlow Windows desktop test build

## 使用

1. 解压整个 ZIP，双击 **VoiceFlow.exe**。不要单独复制 EXE。
2. 设置页在独立的 Tauri 桌面窗口内打开；不再启动外部浏览器或 PowerShell 脚本。
3. 关闭窗口后仍可听写。左键点击系统托盘图标重新打开设置；右键选择 **Quit VoiceFlow / 退出** 完全退出。退出前完成录音。
4. 首次使用在 Settings 中确认快捷键和 Provider。模型与 Python 已包含，ASR 无需联网下载；Provider 功能需要网络与有效配置。
5. 不要同时运行旧测试包或开发版，它们可能争用快捷键。

Windows x64。需要 Microsoft Edge WebView2 Evergreen Runtime；缺失时请联系 IT 安装：
https://developer.microsoft.com/microsoft-edge/webview2/
这个便携包不附带 WebView2，不会自行下载安装或更改系统策略。

用户数据、日志和 WebView2 缓存在 `%LOCALAPPDATA%\VoiceFlow Speech Input`；桌面后台的临时音频和诊断报告使用其 `temp` 子目录，避免混入开发版的旧临时报告。API key 由 Windows Credential Manager 保存。退出应用不会删除设置。便携目录不携带个人历史和凭据。

桌面入口固定使用自身目录中的 `input-host.exe`、`runtime/python/python.exe`、`runtime/asr/worker.py` 和模型目录，覆盖继承的 ASR 路径环境变量。后台沿用经过验证的 localhost Settings 服务，仅加载其自身地址，网页没有 Tauri 文件、命令行或进程权限。后台先等待桌面完成 Windows Job Object 接管，再开始工作；退出时先通知后台正常收尾，3 秒后仍未结束则清理整个进程树。

## 公司电脑 / 比赛演示

便携包省去安装步骤，但仍包含 EXE、DLL 和 Python。公司应用控制可能拦截这些文件；ZIP 不能绕过应用控制。提前向 IT 确认允许的交付形式，必要时提供 ZIP 哈希、文件清单和申请批准。当前为未签名测试版，不保证被公司策略接受。

在公司电脑上提前验证：WebView2、麦克风权限、快捷键、中文与英文听写、长录音、多行粘贴、选区改写，以及公司允许使用的 Provider 网络连接。建议用非敏感演示文本进行比赛测试。

## 开发与打包

Rust：`cargo build --release --locked -p input-host -p voiceflow-desktop`。

生成便携包：`scripts/build-desktop-portable.ps1 -RuntimeBundle dist/VoiceFlow-portable-review`。RuntimeBundle 是已验证的 Python/模型来源，不是运行时依赖；输出包含独立副本。

调试桌面窗口：设置 `VOICEFLOW_DESKTOP_ROOT` 为一个完整桌面便携目录的绝对路径，然后执行 `cargo run -p voiceflow-desktop`。这个覆盖项仅在 debug 构建生效。可追加 `-- --settings-only` 仅启动设置窗口，不注册录音快捷键。前端无需 Node 运行时；开发工具 npm 仅用于 Tauri CLI 和图标生成。修改 UI 后将资源同步到调试目录并刷新窗口，修改 Rust 后重新编译。

安装包（NSIS setup.exe / MSI）是另一个交付形式，必须把模型和 Python 一起纳入安装资源。本流程目前交付便携 ZIP，并不把一个裸 EXE 当成完整安装包。
