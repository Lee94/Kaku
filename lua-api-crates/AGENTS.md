# LUA-API-CRATES KNOWLEDGE BASE

**Context:** Rust-to-Lua API bindings for user configuration scripts.

## Scope

`lua-api-crates/` exposes runtime capabilities to Lua, including:
- mux and pane controls
- window and spawn helpers
- ssh and system integration helpers
- utility modules like logging, time, and filesystem

## Where to Look

- `lua-api-crates/mux/`: pane/tab control APIs
- `lua-api-crates/window-funcs/`: window-level APIs
- `lua-api-crates/spawn-funcs/`: process spawn APIs
- `lua-api-crates/ssh-funcs/`: SSH-related Lua APIs

## Practical Rules

- API behavior here is user-facing and breaking-change sensitive.
- Keep naming and semantics consistent with existing Lua surface.
- Validate changes against real config usage patterns.
- Be careful with user-var and event-facing APIs, since GUI hooks and config reload signals rely on those event paths.
- Keep proxy, config reload, and user-var event semantics stable for AI and automation workflows that observe Lua-facing state.

## Cross-References

- **[`mux/AGENTS.md`](../mux/AGENTS.md)** - Exposes mux controls to Lua
- **[`config/AGENTS.md`](../config/AGENTS.md)** - Lua config execution environment
- **[`kaku-gui/AGENTS.md`](../kaku-gui/AGENTS.md)** - Window functions and GUI integration
