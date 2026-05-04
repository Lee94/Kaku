# KAKU-GUI KNOWLEDGE BASE

**Context:** GUI crate for rendering, window lifecycle, user interaction, AI chat, and the `k` helper binary.

## Scope

`kaku-gui` is responsible for:
- terminal window rendering
- input and mouse handling
- tab/pane UI behavior
- startup lifecycle and app event flow
- AI chat overlays, approval UI, inline status, syntax highlighting, and assistant transport
- the `k` helper binary under `src/bin/k.rs`

## Where to Look

- `src/termwindow/mod.rs`: core window state and action dispatch
- `src/termwindow/render/`: rendering pipeline
- `src/termwindow/mouseevent.rs`: mouse and drag behavior
- `src/termwindow/webgpu.rs`: surface resize and present mode handling
- `src/frontend.rs`: app lifecycle and macOS integration events
- `src/overlay/`: launcher and overlays
- `src/overlay/ai_chat/`: AI chat overlay, markdown rendering, syntax highlighting, and Waza integration
- `src/ai_chat_engine/`: AI chat engine, approval, and compaction flow
- `src/ai_client.rs`, `src/ai_gemini.rs`, `src/ai_remote.rs`, `src/ai_tools.rs`: provider and tool integration
- `src/cli_chat/`: command-line chat flow
- `src/thread_util.rs`: macOS worker thread lifetime helpers
- `src/tabbar.rs`: tab bar behavior

## Practical Rules

- Prefer existing helper methods over re-implementing logic inline.
- Keep rendering decisions in rendering modules, not scattered across event code.
- Be careful with drag and wheel interactions; regressions are user-visible quickly.
- Keep initial window dimension math in `TermWindow::new_window()` consistent with `get_os_border()` behavior.
- Route menu-driven update flows through `run_kaku_update_from_menu()` in `src/frontend.rs`.
- Keep `KAKU_CONFIG_CHANGED` fast path in `emit_user_var_event()` before pane ownership checks.
- Pass fullscreen state into `TabBarState::new()` and preserve fullscreen-specific title button spacing behavior.
- Keep AI overlay state changes cheap enough for live terminal interaction; avoid blocking render/input loops on provider calls.
- Preserve approval prompts and inline AI status placement when touching AI chat or shell flows.
- Wrap macOS worker thread spawns in the existing autorelease-pool helper so shutdown does not reintroduce use-after-free crashes.

## Known Pitfalls

- During window drag, terminal move/wheel events may need suppression.
- WebGPU surface reconfigure should happen only on meaningful state changes.
- Finder "open with" behavior is handled in `frontend.rs` event flow.
- Option+Click cursor movement fires only when: no mouse grab, no alt screen, click is on the same row as the cursor. It sends Left/Right arrow sequences proportional to the column delta; see `mouseevent.rs`.
- Block cursor height uses `natural_cell_height` (ignoring `line_height`) so Nvim visual selections match WezTerm proportions.
- Top border clearance for macOS integrated buttons is state-sensitive:
  - top tab bar visible -> add small gap
  - top tab bar hidden or bottom tab bar -> add larger clearance for traffic lights
- Overlay geometry must update when panes are split, resized, closed, or moved.
- Alternate-screen wheel behavior is config-controlled; do not hard-code one terminal behavior across normal and alternate screens.
- Inline `#` AI query status belongs below the buffer, not as a toast, so terminal content remains stable.
- `fast_model` and proxy config are part of the AI UX contract; verify both when changing provider selection or transport.

## Cross-References

- **[`mux/AGENTS.md`](../mux/AGENTS.md)** - Tab/pane abstractions consumed by GUI
- **[`termwiz/AGENTS.md`](../termwiz/AGENTS.md)** - TUI primitives used in overlays
- **[`config/AGENTS.md`](../config/AGENTS.md)** - Config loading and reload signals
