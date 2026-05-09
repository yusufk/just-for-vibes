# just-for-vibes (j4v)

A TUI web browser. Any website, rendered as a pure terminal interface.

## Concept

- Homepage: Google-style search page (input + Search + I'm Feeling Lucky)
- Search results: real websites, clickable
- Clicking a result: proxied through a Cloudflare Worker that converts HTML → TUI-renderable format
- Usable by humans AND AI agents (it's already text)

## Architecture

```
┌──────────────────────────────┐
│  j4v TUI Client (terminal)   │
│  - Search homepage            │
│  - Renders TUI pages          │
│  - Handles navigation/forms   │
└──────────────┬───────────────┘
               │ HTTPS
┌──────────────▼───────────────┐
│  Cloudflare Worker (proxy)    │
│  - /search?q=...  → results  │
│  - /browse?url=... → convert  │
│  - Fetches target page        │
│  - HTML → TUI wire format     │
└──────────────────────────────┘
```

## Stack

- **Client**: Rust + ratatui (instant startup, zero GC, single binary)
- **Proxy**: Cloudflare Worker (JS/TS)
- **Search**: DuckDuckGo (free, no API key) or Google via SerpAPI
- **Wire format**: JSON (structured elements the client renders as widgets)

## Wire Format (draft)

```json
{
  "title": "Page Title",
  "url": "https://example.com",
  "elements": [
    {"type": "heading", "level": 1, "text": "Welcome"},
    {"type": "text", "content": "Some paragraph text..."},
    {"type": "link", "text": "Click here", "href": "/page2"},
    {"type": "input", "name": "q", "placeholder": "Search..."},
    {"type": "button", "text": "Submit", "action": "/search"},
    {"type": "list", "items": ["Item 1", "Item 2"]},
    {"type": "image", "alt": "Photo description", "src": "..."}
  ]
}
```

## Milestones

1. **Worker MVP**: `/search` endpoint returns DuckDuckGo results as JSON
2. **Worker Browse**: `/browse?url=` fetches + converts HTML → wire format
3. **TUI Client**: Renders homepage, search input, results list
4. **TUI Browse**: Renders converted pages with links, text, headings
5. **Navigation**: Click links, go back, address bar
6. **Forms**: Submit search forms, login forms
7. **Polish**: Loading states, error handling, bookmarks
