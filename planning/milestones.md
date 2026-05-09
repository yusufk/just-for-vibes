# Milestones

## M1: Worker — Search Endpoint
- [ ] Cloudflare Worker project scaffold (`wrangler init`)
- [ ] `GET /search?q=...` → DuckDuckGo results as JSON
- [ ] Return structured search results (title, url, snippet)
- [ ] Deploy to `j4v.yusufk.workers.dev`

## M2: Worker — Browse/Convert Endpoint
- [ ] `GET /browse?url=...` → fetch target page
- [ ] HTML → wire format conversion (using HTMLRewriter or parse)
- [ ] Strip ads, scripts, styles
- [ ] Extract headings, paragraphs, links, forms, tables
- [ ] Handle relative URLs (resolve to absolute)

## M3: TUI Client — Homepage
- [ ] Python + Textual project setup
- [ ] Google-style homepage: logo, search input, two buttons
- [ ] Submit search → call Worker `/search` endpoint
- [ ] Display results as clickable list

## M4: TUI Client — Page Rendering
- [ ] Render wire format elements as Textual widgets
- [ ] Links are focusable/clickable (navigate to new page)
- [ ] Back/forward navigation stack
- [ ] Address bar showing current URL

## M5: Forms & Interaction
- [ ] Render form inputs as TUI text inputs
- [ ] Submit forms via Worker (POST proxy)
- [ ] Select/dropdown as TUI picker
- [ ] Basic cookie jar for session continuity

## M6: Polish
- [ ] Loading spinner during fetch
- [ ] Error pages (404, timeout, blocked)
- [ ] Bookmarks (local file)
- [ ] History (local file)
- [ ] `--url` flag to open directly to a page
- [ ] `--agent` mode: JSON output only (no TUI, for AI agents)
