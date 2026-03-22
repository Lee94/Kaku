# V0.8.0 Fish 🐟

<div align="center">
  <img src="https://raw.githubusercontent.com/tw93/Kaku/main/assets/logo.png" alt="Kaku Logo" width="120" height="120" />
  <h1 style="margin: 12px 0 6px;">Kaku V0.8.0</h1>
  <p><em>A fast, out-of-the-box terminal built for AI coding.</em></p>
</div>

### Changelog

1. **Fish Shell Support**: Full fish shell integration via `kaku init`, including Starship prompt, Yazi launcher, theme sync, and conf.d entrypoint.
2. **Bell Tab Indicator**: Background tabs show a bell prefix when tasks finish, with optional Dock badge and toggleable tab prefix.
3. **Remember Last Directory**: Kaku restores the last working directory when opening new tabs or windows.
4. **Update & Doctor in Tabs**: `kaku update` and `kaku doctor` now open in a dedicated tab instead of blocking the current session.
5. **Basename-only Tab Titles**: New `tab_title_basename_only` option to show just the directory name instead of the full path.
6. **Scrollback Fix**: Fixed viewport jumping to top during rapid output, and snapping to bottom after scrolling up.
7. **Bug Fixes & Stability**: Fixed window closing instead of hiding, Cmd+Click links, clipboard image paste, OpenGL flush on sleep/wake, emoji width, SSH alias conflicts in zsh, and viewport jumping when using Claude Code. Close dialogs now support Enter to confirm and Esc to cancel.

### 更新日志

1. **Fish Shell 完整支持**：`kaku init` 现支持 fish shell 完整引导，含 Starship 提示符、Yazi 启动器、主题同步及 conf.d 入口。
2. **铃声标签指示器**：后台标签任务完成时显示铃声前缀，支持可选 Dock badge 和标签前缀开关。
3. **记住上次目录**：新标签和新窗口打开时自动恢复上次工作目录。
4. **Update/Doctor 在标签中运行**：`kaku update` 和 `kaku doctor` 在独立标签中打开，不阻塞当前会话。
5. **仅显示目录名标签**：新增 `tab_title_basename_only` 选项，只显示目录名而非完整路径。
6. **滚动修复**：修复快速输出时 viewport 跳到顶部，以及往上滚动后自动跳回底部的问题。
7. **Bug 修复与稳定性**：修复窗口实际关闭而非隐藏、Cmd+Click 链接打开、剪贴板图片粘贴、sleep/wake 时 OpenGL flush、emoji 宽度渲染、zsh 中 SSH alias 冲突，以及使用 Claude Code 时 viewport 异常跳动的问题。关闭确认弹窗现支持回车确认、Esc 取消。

Special thanks to @mystersu, @ddotz, @rookie-ricardo, @s010s, @anzksdk, @cynosurech, and @XinCao for their contributions to this release.

> https://github.com/tw93/Kaku
