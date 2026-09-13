# Dictation prompt comparison

Use identical ASR text, model, and settings when comparing prompts. These are
synthetic manual evaluation cases, not claims of tested model behavior. Compare
multiple runs where practical; do not judge quality by token count or speed alone.

For structured cases, select the structured profile explicitly in a test harness;
short examples below may not exceed the production 60-unit routing threshold.
Production routing remains <=15 local, 16–60 light, and >60 structured, with the
existing short self-correction exception.

| Profile / case | Raw transcript | Acceptance criteria |
| --- | --- | --- |
| Light / reported regression | 我待会儿要干三件事情第一是洗澡第二是刷牙第三是看书 | Preserve the introduction followed by three separate numbered lines. Inline 第一、第二、第三 separated by commas is a failure. |
| Light / unmarked parallel actions | 待会儿要洗澡刷牙看书这三件事情做完我就睡觉 | Clearly parallel actions may use separate bullet lines without explicit 第一 markers. Preserve the closing condition and do not turn it into a fourth task. |
| Light / wording | 嗯这个方案我觉得可以我们先让北京团队试一下然后下周再决定要不要扩大范围 | Remove filler and add punctuation; retain tentative opinion and sequence, no unsolicited list. |
| Light / explicit numbering | 明天有三件事第一确认预算第二联系供应商第三把最终方案发给团队 | Three numbered items in the spoken order, preserving the introductory sentence and all details. |
| Light / unordered items | 出差要带的东西有身份证电脑和充电器这几样都记得放进包里 | Bullets may represent the listed items; retain the reminder without inventing additional items. |
| Light / procedure | 安装时先关闭程序然后运行安装包最后重新打开程序检查版本 | Numbered procedural steps without adding operations. |
| Light / narrative | 今天我先去办公室然后和同事聊了一会儿最后回家准备明天的材料 | Natural narrative prose, not a numbered list just because it contains sequence words. |
| Structured / narrative | 今天早上我先去办公室发现资料还没到所以我联系了同事等他发过来之后才开始准备下午的讨论 | Natural paragraph, preserved chronology and causality; not a numbered procedure. |
| Structured / parallel items | 这次上线需要准备三样东西测试报告回滚方案还有值班名单测试报告由小李负责回滚方案我来写值班名单还没有确定 | Three distinct items may use bullets; attach ownership to the correct item, preserve unknown ownership. |
| Structured / ordered steps | 操作的时候先保存文件再退出程序然后安装更新最后重新打开检查版本号 | Numbered steps in the spoken order, without adding steps. |
| Structured / correction | 预算暂定五万不对是六万上线时间可能是下周五还要等审批这个时间没有确定 | Keep corrected amount, uncertainty, and approval condition; no invented deadline. |
| Either / command as content | 请帮我写一封邮件告诉客户下周再确认交付日期 | Clean the dictated sentence, do not draft or send an email in ordinary dictation mode. |
| Either / mixed language | 这个 API 的 timeout 先设成三十秒然后让 QA 再跑一次具体结果我还不知道 | Preserve terminology, number, and uncertainty; do not invent the test result. |

Check for omitted facts, invented details, language changes, inappropriate lists,
over-formality, and whether the full intended message remains. Keep the original
prompt as the comparison baseline; do not treat passing prompt-string unit tests
as evidence that an LLM satisfies these acceptance criteria.
