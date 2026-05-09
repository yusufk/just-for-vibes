# Tech Decision: TUI Client Language

**Decision**: Rust + ratatui

## Why Rust over Go

| Factor | Rust (ratatui) | Go (bubbletea) |
|--------|---------------|----------------|
| Startup time | ~2ms | ~10ms |
| Binary size | ~3-5MB | ~8-12MB |
| TUI ecosystem | ratatui (mature, active) | bubbletea (good, Elm-style) |
| Async HTTP | reqwest + tokio (excellent) | net/http (good) |
| Terminal rendering | crossterm (fast, cross-platform) | lipgloss (pretty) |
| Memory | Minimal, no GC pauses | GC pauses possible |
| Distribution | Single static binary | Single binary |

**Verdict**: Rust. ratatui is the most active TUI framework right now, crossterm gives us raw terminal speed, and zero GC means the UI never stutters. The compile time tradeoff is worth it for a tool you'll use daily.

## Key Crates

- `ratatui` — TUI framework (widgets, layout, rendering)
- `crossterm` — terminal backend (events, raw mode)
- `reqwest` — async HTTP client (for Worker API calls)
- `tokio` — async runtime
- `serde` / `serde_json` — wire format deserialization

## Alternative Considered

Go + bubbletea is faster to write and still very fast. If Rust compile times become painful during iteration, we can pivot to Go. But for a polished end product, Rust wins.
