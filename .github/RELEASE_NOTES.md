# V0.12.0 Sharper ✂️

<div align="center">
  <img src="https://raw.githubusercontent.com/tw93/Kaku/main/assets/logo.png" alt="Kaku Logo" width="120" height="120" />
  <h1 style="margin: 12px 0 6px;">Kaku V0.12.0</h1>
  <p><em>A fast, out-of-the-box terminal built for AI coding.</em></p>
</div>

### Changelog

1. **Session Scrollback**: Continue-where-you-left-off now restores each pane's scrollback history (up to 1,500 lines), reflowing when the column width changed. Running processes cannot be revived, but the previous text is back when you reopen.
2. **SmartPrompt**: Cmd+Q now defaults to `SmartPrompt`, quitting instantly when every pane is at a shell prompt and asking first when an agent or editor is still running. Set `window_close_confirmation` in your config to override.
3. **macOS Window**: Light/Dark flips refresh all windows in a single pass, the red-dot button cleanly exits Space fullscreen, maximized-window drag waits for real movement before detaching, title-bar single-click no longer false-triggers maximize, and tab click regions update immediately after rename.
4. **AI Prompt Rebuild**: The chat system prompt is split into six topical fragments with versioned metadata headers and a CI gate, so the prompt chain stays auditable and stable for cache hits.
5. **Smarter Chat Loop**: `/suggest`, older-history fold-into-summary, JSON-output session titles, and a webfetch summarizer all route to the configured fast model and keep the context window tight.
6. **AI Shell Hardening**: The `#` quick-fix flow gains command-injection detection on known-risky patterns, externalized prompts, and intent-based dispatch between command synthesis, explain, and lookup paths.
7. **Tool Sandbox Audit**: File-access policy ships with thorough symlink-escape and credential-path tests, and now blocks credential file names (`.env`, `*.pem`, `*.key`, SSH private keys) in any directory while still allowing committed `.env.example` templates.
8. **Document Open**: PDFs, images, audio, video, archives, and Office documents launch in their default app instead of being grabbed by VS Code.
9. **Window Polish**: Hollow cursor on the unfocused active pane, non-fancy tab bar top inset and cell height, and a stale "Restart to Update" menu item are all cleaned up.
10. **Font Scaling**: Prompt redraws are skipped while the font scale is settling, and PTY resizes flush only after the cell dimensions stabilize.
11. **`kaku chat` Overlay**: Every invocation reliably retriggers the AI chat overlay, even when the user-var value would otherwise be deduped.
12. **Tidy**: Simplified Chinese localization is removed (the `language` option is still accepted as a deprecated field), `smart_tab_mode` is added, zsh managed-comment highlight is brighter in dark themes, dependencies are audit-clean, and new CI gates cover logs, clippy, and prompt metadata.
13. **Codex Backend**: Kaku Assistant can use your existing `codex` (ChatGPT) login as a backend through the Responses API, with no separate API key. Opt in by setting Auth Type to `codex` in `kaku ai` and picking a Codex model such as `gpt-5-codex`.
14. **AI Chat Rendering**: The chat overlay now repaints from its own output, so streamed replies and the loading indicator update in real time instead of waiting for a click, and every chat loader shares one sparkle spinner.

### 更新日志

1. **会话历史恢复**：接续上次会话时现在会恢复每个面板的滚动历史（最多 1,500 行），列宽变化时自动重排。正在运行的进程无法恢复，但上次的输出文本回来了。
2. **SmartPrompt**：Cmd+Q 默认改为 `SmartPrompt`，所有面板都在 shell 提示符时直接退出，仍有 agent 或编辑器在运行时先询问。可在配置中设置 `window_close_confirmation` 覆盖。
3. **macOS 窗口**：浅/深色切换会一次性刷新所有窗口，红点按钮干净退出 Space 全屏，最大化窗口拖拽需真正移动才脱离，标题栏单击不再误触最大化，重命名标签页后点击区域立即更新。
4. **AI 提示重构**：聊天系统提示拆成六个主题片段，每段带版本化的 metadata 头，并加上 CI 校验，提示链可审计也对缓存友好。
5. **对话回路更轻**：`/suggest`、旧历史 fold 成 summary、JSON 输出的会话标题、webfetch 摘要全部走配置中的 fast model，让上下文更紧。
6. **AI Shell 加固**：`#` 快速修复流加入命令注入检测、提示外置，并按意图在命令合成、解释、查询之间分发。
7. **工具沙箱审计**：文件访问策略补充了软链逃逸与凭证路径的完整测试，并新增按文件名拦截凭证文件（`.env`、`*.pem`、`*.key`、SSH 私钥），任意目录均生效，同时放行 `.env.example` 这类示例模板。
8. **文档默认打开**：PDF、图片、音视频、压缩包、Office 文档都走系统默认 app，不再被 VS Code 抢走。
9. **窗口细节**：非聚焦活动 pane 的空心光标、非 fancy 标签栏的顶部内距与 cell 高度、菜单里残留的"重启更新"项都做了收敛。
10. **字体缩放**：缩放未稳定前跳过 prompt 重绘，PTY 大小调整在 cell 维度稳定后才一次性下发。
11. **`kaku chat` 浮层**：每次调用都能稳定触发 AI 聊天浮层，不再被 UserVar 去重机制吞掉。
12. **轻装**：简体中文本地化整体移除（`language` 字段仍作为 deprecated 字段保留兼容），新增 `smart_tab_mode`，zsh 暗色主题下托管注释的高亮更清晰，依赖 audit clean，新增日志、clippy、提示元数据三道 CI 门禁。
13. **Codex 后端**：Kaku Assistant 现在可以直接用你已有的 `codex`（ChatGPT）登录作为后端，走 Responses API，无需另配 API key。在 `kaku ai` 里把 Auth Type 设为 `codex`、选 `gpt-5-codex` 这类 Codex 模型即可启用。
14. **AI 聊天渲染**：聊天浮层现在由自身输出驱动重绘，流式回复和加载动画实时刷新，不再需要点一下才更新；所有聊天加载指示统一成同一个 sparkle spinner。

> https://github.com/tw93/Kaku
