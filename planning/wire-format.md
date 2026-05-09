# Wire Format Specification

## Overview

The Cloudflare Worker converts HTML pages into a JSON wire format that the TUI client renders as terminal widgets.

## Element Types

| Type | TUI Rendering | Properties |
|------|---------------|------------|
| `heading` | Bold/colored text, sized by level | `level`, `text` |
| `text` | Wrapped paragraph | `content` |
| `link` | Highlighted, clickable (Tab to focus) | `text`, `href` |
| `input` | Text input widget | `name`, `placeholder`, `value` |
| `button` | Clickable button | `text`, `action`, `method` |
| `list` | Bulleted/numbered list | `items`, `ordered` |
| `image` | Alt text in brackets: [Photo of...] | `alt`, `src` |
| `table` | Terminal table (rich) | `headers`, `rows` |
| `separator` | Horizontal rule | — |
| `form` | Groups inputs + button | `action`, `method`, `children` |
| `select` | Dropdown/picker | `name`, `options`, `selected` |

## Conversion Rules

1. Strip all CSS, JS, ads, tracking
2. Preserve semantic structure (headings, paragraphs, lists, links)
3. Forms become interactive TUI widgets
4. Images become their alt text
5. Tables preserved as structured data
6. Navigation/header/footer extracted separately
7. Max depth: flatten deeply nested divs

## Search Results Format

```json
{
  "type": "search_results",
  "query": "1/4 inch elbow fitting",
  "results": [
    {
      "title": "Result Title",
      "url": "https://...",
      "snippet": "Description text..."
    }
  ]
}
```

## Open Questions

- How to handle JavaScript-rendered pages? (Worker can't execute JS)
- Cookie/session management for authenticated browsing?
- Rate limiting on the Worker to prevent abuse?
- Should images be converted to ASCII art optionally?
