# 发布前代码审查

审查日期：2026-09-11 至 2026-09-12（Asia/Singapore）。
基线：本地 `main`，`65ae03deed1bff08bd91b38349702567a6b57d1e`，无远程仓库。
初审结果：DONE_WITH_CONCERNS。以下 R1–R11 保留初审证据和当时行号；修复后的状态见下一节。

## 修复与便携包验证（2026-09-12）

用户授权逐项修复，并选择先交付解压即用的便携验证包。R1–R11 已完成代码、测试或文档修复：

| 项目 | 修复结果 |
| --- | --- |
| R1 | 多行粘贴失败明确返回失败并保留结果，不再回退为 Enter 按键输入。 |
| R2 | 不完整 HTTP body 返回错误，不越界切片；实际服务收到截断请求后仍可响应。 |
| R3 | 等待录音停止失败时显式停止采集；测试验证下一次录音可继续。 |
| R4/R5 | Python、worker、模型和 UI 从程序相对目录加载；运行时镜像、WebView2 缓存写入用户目录。便携目录内附独立 Python 和模型。 |
| R6 | 选区复制、读取或等待出错时均尝试恢复原剪贴板；覆盖错误路径。 |
| R7 | 重叠片段按 token 时间戳及重叠区中点分配文本，保留真实重复词。旧 worker 缺时间戳时明确失败。真实长录音边界听感仍待人工验收。 |
| R8 | ready 超时、无效 JSON、错误消息和初始化失败均终止并回收子进程。 |
| R9 | README、architecture、spec、snapshot 和 UI 说明同步当前路由、五项导航及部署布局。 |
| R10 | UI 测试使用隔离 fixture、当前导航和实时坐标；修复 provider 提示文字的 CSS 宽度冲突。 |
| R11 | 镜像路径由实际加载的 settings 路径推导；完整测试前后源码镜像哈希一致。 |

仅测试使用的四个辅助入口加上 `#[cfg(test)]`，避免把测试包装函数编入发布程序。没有进行大规模重构。

验证证据：

- `cargo test --workspace --offline -- --test-threads=1`：490 项通过，5 项默认忽略；源码镜像未变化。
- 另启用 `failed_ready_reaps_the_child` 与 `background_prefetch_then_final_tail_uses_same_worker`：2 项通过，使用包内 Python。
- 所有前端 `*.test.js`：24 项通过，含两个浏览器测试。
- Release 构建成功；未替换或终止用户正在运行的 debug 程序。
- 包内 Python、真实 ASR 模型完成中文样本转写：41 个 token / 41 个时间戳。
- `scripts/verify-portable.ps1`：从仓库和便携目录之外启动 Settings，验证动态状态、截断请求恢复、包目录文件哈希不变。
- `dist/VoiceFlow-portable-review.zip`：182,429,720 字节；归档中 1,065 个文件的 SHA-256 与 manifest 一致，两个运行时脚本均为空占位。未包含 settings/history/env/test/log 文件。
- ZIP SHA-256：`C806FFDA04D0F0B0203A43FB0641235217D1BB8078889F554D0E7A78C26F42F3`。

当前交付为本机验证过的便携测试包。干净 Windows 环境、真实麦克风长录音、目标应用多行粘贴和选区改写仍需验收，不能用本报告代替这些检查。包内使用空运行时占位文件，排除开发机历史、凭据、`.env.local` 和诊断报告。用法见 `docs/portable-usage.md`。

## 初审记录（以下为修复前状态）

以当前实现为准，文档用于交叉核对。重点检查主机录音生命周期、ASR 分段/预识别、文本插入、Settings HTTP 服务、资源部署、文档和现有测试。不是逐行穷尽审计，也没有进行安装包或真实目标应用的端到端验收。

## P1：优先处理

### R1 多行粘贴失败后可能误发送消息或执行命令（9/10）

- `crates/input-host/src/windows_insertion.rs:151`：`Err(clipboard_error) => match direct(text)`。
- 同文件 `318`：`inputs.push(build_virtual_key_input(VK_RETURN as u16, 0));`。
- 剪贴板含图片等不支持备份的格式时，`snapshot_clipboard` 会拒绝粘贴；多行内容随即走直接输入，把换行转为真实 Enter。在 Enter 表示发送/执行的目标应用中，这会触发动作，而不是仅输入文本。
- 建议：多行粘贴失败应保留结果并报告失败，不自动发送 Enter；针对图片剪贴板、聊天输入框和终端验收。现有 `unsafe_clipboard_state_falls_back_to_direct_unicode_send_input` 测试反而固化了此行为。
- 证据为代码控制流；没有实际发送消息或执行终端文本。

### R2 截断 HTTP 请求导致 Settings 服务 panic（10/10）

- `crates/input-host/src/settings_server.rs:531`：读取返回 0 后直接结束循环。
- 同文件 `577`：`let body = buffer[header_end..header_end + content_length].to_vec();`。
- 客户端声明 `Content-Length: 20`，实际只发送 `{}` 后关闭发送端，会对不足长度的 buffer 切片。发生在 token 校验之前；服务循环没有捕获 panic。
- 已提取原解析函数，在临时目录用本地 socket 隔离复现：`truncated_body_panicked=true`，错误为 `range end index 91 out of range for slice of length 73`。未攻击正在运行的服务。
- 建议：在 EOF 和切片前验证实际正文长度，返回协议错误；加入短正文、零正文、提前断连的回归测试。

### R3 录音停止等待出错会遗留麦克风捕获（9/10）

- `crates/input-host/src/host.rs:920` 调用 `await_recording_stop(...)`，错误传播在 `929` 提前返回，跳过 `933` 的 `self.audio_adapter.stop(session_id)`。
- `windows_hotkeys.rs:267` 的 `WAIT_FAILED` 是真实错误来源。`cpal_audio.rs:90` 在 `active_capture` 已占用时拒绝下一次录音；主循环只记录会话失败然后继续。
- 影响：捕获流继续存在，后续录音报 `audio capture is already active`，需要重启恢复。
- 建议：录音启动后的所有退出路径都清理捕获流；用失败热键 mock 验证 stop 被调用且下一次 start 成功。未模拟 Win32 故障。

### R4 ASR 仍绑定开发机绝对路径（10/10）

- `crates/speech-engine/src/worker.rs:58`：`python_executable: PathBuf::from(r"D:\LLM\sensevoice_test\venv\Scripts\python.exe")`。
- `59`、`60` 的 worker 和调试脚本也在同一外部目录。实际 `SpeechEngine::with_local_worker()` 使用此默认配置。
- 影响：把当前程序复制到普通用户机器，ASR 找不到 Python/worker；仓库本身也没有可独立发布的完整 worker/model 资产。
- 建议：先确定运行时、worker、模型的随包布局及配置入口，再验证干净机器启动。这是文档已承认的打包待办，不是新回归。

### R5 UI 资源依赖源码路径，且要求安装资源目录可写（10/10）

- `crates/input-host/src/app_paths.rs:5` 仅判断当前工作目录下的 `apps`；`10` 回退到 `env!("CARGO_MANIFEST_DIR")`。
- `main.rs:1808` 启动强制写 Settings 镜像；`settings_bridge.rs:450` 对静态资源路径执行 `fs::write`。overlay 构造也会写镜像。
- 影响：从其它工作目录或另一台机器启动会找错资源；即使资源随包附上，安装到不可写目录时也可能启动失败。
- 建议：资源从 exe 相对路径或嵌入资源加载；生成状态放用户数据目录。需要验证任意工作目录、普通用户、只读安装目录。

## P2：正确性和维护问题

### R6 选区复制失败会跳过剪贴板恢复（9/10）

- `crates/input-host/src/windows_insertion.rs:382` 先清空剪贴板，随后 `send_ctrl_shortcut(b'C')` 和 `read_clipboard_text(...)` 的错误通过 `?` 提前返回。
- 正常分支使用 `finalize_selected_text_capture` 恢复，但上述错误路径绕过它。
- 影响：复制快捷键失败或读取剪贴板出错后，用户原有剪贴板可能丢失。
- 建议：把复制/读取动作包在统一结果处理中，保证修改剪贴板后的失败也进入恢复逻辑；故障注入验证。未操作用户剪贴板。

### R7 长录音硬切重叠可能产生重复转写（8/10）

- `crates/speech-engine/src/worker.rs:1208` 将前段延长到 `cut + overlap_samples`，`1212` 让下一段从 `cut` 开始。重叠常量是 400 ms。
- 同文件 `353` 至 `358` 将所有 transcript 直接 `.join(" ")`，没有边界对齐。
- 相邻片段分别返回“我们明天发布”和“发布新版本”时，会得到“我们明天发布 发布新版本”。实际频率取决于 ASR 输出；后处理不能作为可靠去重保证。
- 建议：针对硬切边界做对齐策略和真实语音验收，不要全局删除重复词。现有分段测试证明了重叠范围，未证明边界文本不重复。

### R8 worker 初始化失败缺少子进程回收（9/10）

- `crates/speech-engine/src/worker.rs:765` 创建 child；`787` 等待 ready 使用 `?`，后续异常 ready 分支也直接返回。
- 这些退出发生在 child 放入 `WorkerProcess`/进程槽之前，现有 shutdown 路径无法回收它，也没有显式 kill/wait。
- 影响：初始化卡死或超时后，重试可能遗留多个模型进程。
- 建议：初始化失败统一终止并等待子进程；用延迟 ready 的假 worker 验证。未运行卡死模型进程复现。

### R9 路由文档与实现不一致（10/10）

- `README.md:51`、`architecture.md:114` 仍描述 30 字符本地快路径；`spec.md:33` 仍将列表、多行等作为排除规则。
- `crates/shared-protocol/src/lib.rs:354`、`355` 的实际阈值是 15/60，计数单位为汉字、英文单词、数字段；当前路由不沿用文档中的整套旧排除条件。
- `snapshot.md:64` 已记录新阈值；不能用旧 README/spec 作为发布验收依据。
- 建议：按实现统一 README、architecture、spec，并明确哪些是产品期望、哪些是已实现行为。

### R10 现有 UI 测试已经过时（10/10，具体后续失败仍需排查）

- `apps/settings-ui/src/provider-settings-ui.test.js:8` 的列表包含已从子导航移走的 `prompts`，而 `index.html:487` 起实际只有五项，编辑器放在 Advanced 中；测试后面也仍点击旧入口。
- `settings-segmented-control.test.js:38` 使用滚动前缓存的按钮坐标。隔离复现显示 hover 会触发滚动，旧坐标命中另一张卡片；重新取 boundingBox 后命中正确按钮，内存修正后的完整 segmented 测试通过。
- provider 测试仅在内存去掉旧导航期望后，仍在 effective URL 刷新断言失败；原因未确定，不能声称只改一个断言就全绿。
- 建议：让测试匹配实际导航并使用实时坐标，继续排查 provider 刷新，正式测试全绿后再验收。

### R11 Rust 测试污染源码目录的运行时镜像（10/10）

- `settings_server.rs:1137` 的 `valid_token_permits_update_settings` 虽使用临时 Settings/HTML 目录，但生产更新处理在 `206` 调用全局镜像写入函数，没有使用测试 context 的 UI 根目录。
- 本次运行 workspace 测试后，`apps/settings-ui/src/runtime-state.js` 的 settings_path 变为临时测试目录，原实际历史快照被测试数据替换。仅隔离 `VOICEFLOW_SETTINGS_PATH` 不足以隔离输出。
- 建议：让镜像输出路径可注入，并由测试绑定临时目录；测试前后断言工作区文件不变。
- 本次已使用现有 `input-host.exe --print-settings` 从真实设置重新生成 Settings 镜像，确认路径回到用户数据目录。不是原文件逐字节恢复，生成时间等会变化；没有修改真实设置或删除历史。

## 冗余代码与发布资产

Clippy 检查完成，无编译错误，但存在风格/维护告警。`settings_bridge.rs:461`、`494` 两个包装函数以及 `settings_server.rs:41` 的 `local_addr` 只在测试中被使用；适合后续加 `#[cfg(test)]` 或调整测试辅助代码，不应误判为需要删除的产品功能。没有发现值得为此次发布进行大规模重构的冗余模块。

审查开始时两个未提交的 `runtime-state.js` 是生成数据，Settings 镜像包含真实转写历史。这些文件不可直接随包分发。README Packaging Notes 已规定排除它们，但目前尚无发布流程验证此规则。

现有 stash 保留，未 apply/pop/drop。此报告针对当前工作树，不把 stash 作为待发布内容，也不声称其中所有差异都没有价值。

## 验证结果及限制

- `cargo test --workspace --offline`：通过，2 个默认忽略项。
- 单独启用 `worker::tests::background_prefetch_then_final_tail_uses_same_worker`：通过，使用本地假 Python worker，无模型或网络调用。
- Node 普通测试：22 项通过；两个浏览器测试原样运行失败，详见 R10。Playwright 使用本机 Codex bundled 依赖，未安装新依赖。
- `cargo clippy --workspace --all-targets --offline --message-format=short`：完成，有告警。
- HTTP 截断请求隔离复现：确认 panic。
- 尝试 `cargo run ... --print-settings` 重新链接 debug exe 时失败：现有运行中的 exe 被锁定（os error 5）；未停止用户程序。测试二进制和 Clippy 检查均已完成，但不能把这次尝试称为成功的发布构建。
- 没有实际安装包、干净 Windows 环境、真实长录音和目标应用插入验收；没有修复上述生产代码。新增此报告，测试曾更新生成镜像，之后重新生成真实 Settings 镜像。
- in-flight ASR 预识别退出仍可能等待现有请求超时；snapshot 已记录此限制，不列为新缺陷。

## 建议顺序

先修 R1/R2/R3/R6 的插入、崩溃与资源清理，再处理 ASR R7/R8。同步文档和修复测试隔离/旧断言。随后实现 R4/R5 的部署布局及干净发布资产生成，最后在真实安装包上验收。R4/R5 是打包工程本身，代码审查通过不能替代它们。

