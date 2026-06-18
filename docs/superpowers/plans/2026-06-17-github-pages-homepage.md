# GitHub Pages Homepage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a bilingual, GitHub Pages-ready homepage for `redis-rs` with English default, Chinese toggle, and a strong project-focused landing experience while preserving access to the existing documentation overview.

**Architecture:** Reuse the repository's existing root-level Jekyll Pages pipeline. Replace the current root route with a custom homepage assembled from static HTML, CSS, and small vanilla JavaScript, and move the current markdown overview to a secondary page so the old documentation entry remains available.

**Tech Stack:** Jekyll 4, Just the Docs theme, static HTML, CSS, vanilla JavaScript

---

## File Structure

- Modify: `.gitignore`
  - Ignore `.superpowers/` generated during design work.
- Create: `index.html`
  - New root GitHub Pages homepage with semantic sections and `data-i18n` hooks.
- Create: `assets/css/homepage.css`
  - Homepage-only visual design, layout, responsiveness, and motion.
- Create: `assets/js/homepage.js`
  - Translation dictionary, default-English rendering, toggle handling, and optional persistence.
- Modify: `index.md`
  - Convert the existing overview page into a non-root documentation page with a new permalink and nav metadata.
- Modify: `_config.yml`
  - Point auxiliary GitHub link to the real repository and keep the site metadata coherent with the new homepage.
- Verify: local Jekyll output via `bundle exec jekyll build`
  - Confirms the homepage works inside the actual Pages pipeline.

### Task 1: Align the existing Pages entry points

**Files:**
- Modify: `.gitignore`
- Modify: `index.md`
- Modify: `_config.yml`

- [ ] **Step 1: Write the failing verification target**

Document the expected post-change behavior before editing:

```text
Root route `/redis-rs/` should render the new custom homepage.
The existing markdown overview content should remain reachable from a secondary route.
The site-level GitHub link should point to https://github.com/fangpin/redis-rs.
```

- [ ] **Step 2: Inspect the current root entry files**

Run: `sed -n '1,120p' index.md && printf '\n---\n' && sed -n '1,120p' _config.yml && printf '\n---\n' && sed -n '1,120p' .gitignore`

Expected: current root route is driven by `index.md`, site theme is Jekyll/Just the Docs, and `.superpowers/` is not yet ignored.

- [ ] **Step 3: Update `.gitignore` minimally**

Add this line at the end of `.gitignore`:

```gitignore
.superpowers/
```

- [ ] **Step 4: Move the existing overview page off the root route**

Update `index.md` frontmatter from:

```md
---
title: 概览
layout: default
nav_order: 1
---
```

to:

```md
---
title: 概览
layout: default
nav_order: 2
permalink: /overview/
---
```

Keep the existing markdown body content unchanged.

- [ ] **Step 5: Correct the site-level GitHub link**

Update `_config.yml` from:

```yml
aux_links:
  GitHub: https://fangpin.github.io/redis-rs
```

to:

```yml
aux_links:
  GitHub: https://github.com/fangpin/redis-rs
```

- [ ] **Step 6: Run a targeted verification pass**

Run: `sed -n '1,20p' index.md && printf '\n---\n' && rg -n "^\\.superpowers/$|^aux_links:|GitHub:" _config.yml .gitignore`

Expected:
- `index.md` shows `permalink: /overview/`
- `.gitignore` includes `.superpowers/`
- `_config.yml` points GitHub to the repository URL

- [ ] **Step 7: Commit the routing-alignment changes**

```bash
git add .gitignore _config.yml index.md
git commit -m "chore: prepare pages routing for homepage"
```

### Task 2: Create the homepage markup

**Files:**
- Create: `index.html`

- [ ] **Step 1: Write the failing structural expectation**

Document the required homepage sections:

```text
The root homepage must contain:
- hero
- capability modules
- quick start
- architecture strip
- repository/docs CTA
- language toggle with EN default
```

- [ ] **Step 2: Verify that `index.html` does not already exist**

Run: `test -f index.html; echo $?`

Expected: `1`

- [ ] **Step 3: Create the semantic homepage skeleton**

Create `index.html` with this structure:

```html
---
layout: nil
title: redis-rs
permalink: /
---
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>redis-rs | Build Your Own Redis in Rust</title>
    <meta
      name="description"
      content="A Redis clone in Rust with RESP parsing, RDB loading, replication, streams, and transactions."
    />
    <link rel="preconnect" href="https://fonts.googleapis.com" />
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
    <link
      href="https://fonts.googleapis.com/css2?family=Chakra+Petch:wght@500;600;700&family=Manrope:wght@400;500;700;800&display=swap"
      rel="stylesheet"
    />
    <link rel="stylesheet" href="{{ '/assets/css/homepage.css' | relative_url }}" />
  </head>
  <body>
    <div class="page-shell">
      <header class="hero" id="top">
        <nav class="topbar" aria-label="Primary">
          <a class="brand" href="#top">redis-rs</a>
          <div class="topbar-actions">
            <a class="text-link" href="{{ '/overview/' | relative_url }}" data-i18n="nav.overview">Overview</a>
            <a class="text-link" href="https://github.com/fangpin/redis-rs" target="_blank" rel="noreferrer" data-i18n="nav.github">GitHub</a>
            <div class="language-toggle" role="group" aria-label="Language switcher">
              <button class="lang-btn is-active" type="button" data-lang="en">EN</button>
              <button class="lang-btn" type="button" data-lang="zh">中文</button>
            </div>
          </div>
        </nav>

        <section class="hero-content">
          <div class="hero-copy">
            <p class="eyebrow" data-i18n="hero.eyebrow">Protocol. Persistence. Replication.</p>
            <h1 data-i18n="hero.title">Build Your Own Redis in Rust</h1>
            <p class="hero-description" data-i18n="hero.description">
              A toy Redis server in Rust that parses RESP, restores from RDB, replicates leader-follower state, supports streams, and queues transactional commands.
            </p>
            <div class="hero-actions">
              <a class="button button-primary" href="https://github.com/fangpin/redis-rs" target="_blank" rel="noreferrer" data-i18n="hero.primaryCta">View on GitHub</a>
              <a class="button button-secondary" href="#quickstart" data-i18n="hero.secondaryCta">Quick Start</a>
            </div>
          </div>

          <div class="hero-panel" aria-hidden="true">
            <div class="panel-surface">
              <div class="panel-header">
                <span>runtime.signal</span>
                <span>redis-rs</span>
              </div>
              <div class="panel-grid">
                <div class="metric">
                  <span class="metric-label" data-i18n="metrics.protocol">Protocol</span>
                  <strong>RESP</strong>
                </div>
                <div class="metric">
                  <span class="metric-label" data-i18n="metrics.persistence">Persistence</span>
                  <strong>RDB</strong>
                </div>
                <div class="metric">
                  <span class="metric-label" data-i18n="metrics.replication">Replication</span>
                  <strong>PSYNC</strong>
                </div>
                <div class="metric">
                  <span class="metric-label" data-i18n="metrics.streams">Queues</span>
                  <strong>XREAD</strong>
                </div>
              </div>
              <pre class="signal-code"><code>cargo run -- --dir ./db --dbfilename dump.rdb
redis-cli PING
redis-cli INFO replication
redis-cli XADD stream_key "*" foo bar</code></pre>
            </div>
          </div>
        </section>
      </header>

      <main>
        <section class="section capability-section" id="capabilities">
          <div class="section-heading">
            <p class="eyebrow" data-i18n="capabilities.eyebrow">Core capabilities</p>
            <h2 data-i18n="capabilities.title">Built around real Redis subsystems</h2>
          </div>
          <div class="capability-grid">
            <article class="capability-card">
              <p class="card-kicker">01</p>
              <h3 data-i18n="capabilities.protocol.title">Protocol</h3>
              <p data-i18n="capabilities.protocol.body">Parses Redis protocol messages and routes commands through the server runtime.</p>
              <span class="chip">protocol.rs</span>
            </article>
            <article class="capability-card">
              <p class="card-kicker">02</p>
              <h3 data-i18n="capabilities.persistence.title">Persistence</h3>
              <p data-i18n="capabilities.persistence.body">Loads database state from RDB files and restores key metadata.</p>
              <span class="chip">rdb.rs</span>
            </article>
            <article class="capability-card">
              <p class="card-kicker">03</p>
              <h3 data-i18n="capabilities.replication.title">Replication</h3>
              <p data-i18n="capabilities.replication.body">Bootstraps replica handshake, full sync, and replicated write flow.</p>
              <span class="chip">replication_client.rs</span>
            </article>
            <article class="capability-card">
              <p class="card-kicker">04</p>
              <h3 data-i18n="capabilities.streams.title">Streams and transactions</h3>
              <p data-i18n="capabilities.streams.body">Supports stream operations and queued transactional execution patterns.</p>
              <span class="chip">cmd.rs</span>
            </article>
          </div>
        </section>

        <section class="section quickstart-section" id="quickstart">
          <div class="section-heading">
            <p class="eyebrow" data-i18n="quickstart.eyebrow">Run it</p>
            <h2 data-i18n="quickstart.title">Start fast, then probe with redis-cli</h2>
          </div>
          <div class="command-deck">
            <div class="command-card">
              <div class="command-label" data-i18n="quickstart.masterLabel">Master</div>
              <pre><code>cargo run -- --dir /some/db/path --dbfilename dump.rdb</code></pre>
            </div>
            <div class="command-card">
              <div class="command-label" data-i18n="quickstart.replicaLabel">Replica</div>
              <pre><code>cargo run -- --dir /some/db/path --dbfilename dump.rdb --port 6380 --replicaof "localhost 6379"</code></pre>
            </div>
            <div class="command-card command-card-wide">
              <div class="command-label" data-i18n="quickstart.probeLabel">Probe</div>
              <pre><code>redis-cli PING
redis-cli SET foo bar
redis-cli INFO replication
redis-cli XREAD block 0 streams some_key 1526985054069-0</code></pre>
            </div>
          </div>
        </section>

        <section class="section architecture-section" id="architecture">
          <div class="section-heading">
            <p class="eyebrow" data-i18n="architecture.eyebrow">Implementation map</p>
            <h2 data-i18n="architecture.title">How the runtime is assembled</h2>
          </div>
          <div class="architecture-strip">
            <article class="architecture-node">
              <span class="node-file">protocol.rs</span>
              <h3 data-i18n="architecture.protocol.title">Protocol ingestion</h3>
              <p data-i18n="architecture.protocol.body">RESP parsing and request framing for incoming client and replication traffic.</p>
            </article>
            <article class="architecture-node">
              <span class="node-file">server.rs + cmd.rs</span>
              <h3 data-i18n="architecture.command.title">Command routing</h3>
              <p data-i18n="architecture.command.body">Connection handling, command dispatch, and transaction orchestration.</p>
            </article>
            <article class="architecture-node">
              <span class="node-file">storage.rs + rdb.rs</span>
              <h3 data-i18n="architecture.storage.title">State and restore</h3>
              <p data-i18n="architecture.storage.body">In-memory storage backed by RDB parsing and initial database hydration.</p>
            </article>
            <article class="architecture-node">
              <span class="node-file">replication_client.rs</span>
              <h3 data-i18n="architecture.replication.title">Replication flow</h3>
              <p data-i18n="architecture.replication.body">Replica-side handshake, sync start, and downstream command consumption.</p>
            </article>
          </div>
        </section>
      </main>

      <footer class="section footer-cta">
        <div>
          <p class="eyebrow" data-i18n="footer.eyebrow">Source and docs</p>
          <h2 data-i18n="footer.title">Inspect the code, then dive deeper</h2>
        </div>
        <div class="footer-actions">
          <a class="button button-primary" href="https://github.com/fangpin/redis-rs" target="_blank" rel="noreferrer" data-i18n="footer.github">Open GitHub Repository</a>
          <a class="button button-secondary" href="{{ '/README.html' | relative_url }}" data-i18n="footer.readmeEn">Read README</a>
          <a class="button button-secondary" href="{{ '/README_CN.html' | relative_url }}" data-i18n="footer.readmeZh">Read Chinese README</a>
        </div>
      </footer>
    </div>

    <script src="{{ '/assets/js/homepage.js' | relative_url }}"></script>
  </body>
</html>
```

- [ ] **Step 4: Verify the root page structure exists**

Run: `rg -n "data-i18n|hero|capability-grid|quickstart|architecture-strip|language-toggle" index.html`

Expected: all major homepage sections and translation hooks are present.

- [ ] **Step 5: Commit the homepage markup**

```bash
git add index.html
git commit -m "feat: add homepage markup"
```

### Task 3: Implement the homepage styling

**Files:**
- Create: `assets/css/homepage.css`

- [ ] **Step 1: Write the failing visual expectation**

Document what the CSS must establish:

```text
The homepage should render with:
- dark cyber technical palette
- stable responsive layout
- visually distinct hero, capability, quickstart, architecture, and footer sections
- readable command blocks and buttons
```

- [ ] **Step 2: Verify the stylesheet does not already exist**

Run: `test -f assets/css/homepage.css; echo $?`

Expected: `1`

- [ ] **Step 3: Create the stylesheet**

Create `assets/css/homepage.css` with this content:

```css
:root {
  color-scheme: dark;
  --bg: #07111f;
  --bg-elevated: rgba(11, 24, 41, 0.8);
  --bg-panel: rgba(10, 20, 34, 0.92);
  --line: rgba(122, 238, 255, 0.18);
  --line-strong: rgba(122, 238, 255, 0.38);
  --text: #ecf7ff;
  --text-dim: rgba(236, 247, 255, 0.72);
  --accent: #79f0ff;
  --accent-strong: #9dffb8;
  --accent-deep: #163250;
  --shadow: 0 24px 80px rgba(0, 0, 0, 0.4);
  --radius: 20px;
  --content-width: 1180px;
}

* {
  box-sizing: border-box;
}

html {
  scroll-behavior: smooth;
}

body {
  margin: 0;
  min-width: 320px;
  background:
    radial-gradient(circle at top left, rgba(121, 240, 255, 0.18), transparent 28%),
    radial-gradient(circle at 85% 10%, rgba(157, 255, 184, 0.12), transparent 18%),
    linear-gradient(180deg, #050b14, #091426 45%, #07111f 100%);
  color: var(--text);
  font-family: "Manrope", system-ui, sans-serif;
}

body::before {
  content: "";
  position: fixed;
  inset: 0;
  pointer-events: none;
  background-image:
    linear-gradient(rgba(121, 240, 255, 0.06) 1px, transparent 1px),
    linear-gradient(90deg, rgba(121, 240, 255, 0.05) 1px, transparent 1px);
  background-size: 72px 72px;
  mask-image: linear-gradient(180deg, rgba(255, 255, 255, 0.45), transparent 82%);
}

.page-shell {
  width: min(calc(100% - 32px), var(--content-width));
  margin: 0 auto;
  padding: 24px 0 56px;
}

.hero,
.section,
.footer-cta {
  position: relative;
}

.topbar,
.hero-content,
.section,
.footer-cta {
  backdrop-filter: blur(18px);
}

.topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 18px 22px;
  border: 1px solid var(--line);
  border-radius: 18px;
  background: rgba(6, 14, 25, 0.7);
}

.brand,
.text-link,
.button {
  text-decoration: none;
}

.brand {
  color: var(--text);
  font-family: "Chakra Petch", sans-serif;
  font-size: 1.1rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.topbar-actions {
  display: flex;
  align-items: center;
  gap: 14px;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.text-link {
  color: var(--text-dim);
  font-size: 0.95rem;
}

.language-toggle {
  display: inline-flex;
  padding: 4px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.03);
}

.lang-btn {
  border: 0;
  min-width: 60px;
  padding: 9px 14px;
  border-radius: 999px;
  background: transparent;
  color: var(--text-dim);
  font: inherit;
  cursor: pointer;
}

.lang-btn.is-active {
  background: linear-gradient(135deg, rgba(121, 240, 255, 0.22), rgba(157, 255, 184, 0.18));
  color: var(--text);
}

.hero-content {
  display: grid;
  grid-template-columns: minmax(0, 1.1fr) minmax(320px, 0.9fr);
  gap: 28px;
  margin-top: 20px;
  padding: 36px;
  border: 1px solid var(--line);
  border-radius: 28px;
  background:
    linear-gradient(145deg, rgba(8, 21, 36, 0.96), rgba(8, 17, 31, 0.86)),
    radial-gradient(circle at top right, rgba(121, 240, 255, 0.12), transparent 26%);
  box-shadow: var(--shadow);
  overflow: hidden;
}

.hero-copy {
  display: flex;
  flex-direction: column;
  justify-content: center;
}

.eyebrow,
.card-kicker,
.command-label,
.node-file,
.metric-label {
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.eyebrow {
  margin: 0 0 16px;
  color: var(--accent);
  font-size: 0.82rem;
  font-weight: 700;
}

h1,
h2,
h3,
p {
  margin: 0;
}

h1 {
  max-width: 10ch;
  font-family: "Chakra Petch", sans-serif;
  font-size: clamp(3rem, 7vw, 5.8rem);
  line-height: 0.95;
}

.hero-description {
  margin-top: 18px;
  max-width: 58ch;
  color: var(--text-dim);
  font-size: 1.04rem;
  line-height: 1.75;
}

.hero-actions,
.footer-actions {
  display: flex;
  gap: 14px;
  flex-wrap: wrap;
}

.hero-actions {
  margin-top: 28px;
}

.button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 48px;
  padding: 0 18px;
  border-radius: 999px;
  border: 1px solid transparent;
  font-weight: 700;
  transition: transform 180ms ease, border-color 180ms ease, background 180ms ease;
}

.button:hover,
.lang-btn:hover,
.text-link:hover {
  transform: translateY(-1px);
}

.button-primary {
  background: linear-gradient(135deg, rgba(121, 240, 255, 0.2), rgba(157, 255, 184, 0.22));
  color: var(--text);
  border-color: var(--line-strong);
}

.button-secondary {
  background: rgba(255, 255, 255, 0.02);
  color: var(--text);
  border-color: var(--line);
}

.hero-panel {
  display: flex;
  align-items: stretch;
}

.panel-surface {
  width: 100%;
  min-height: 100%;
  padding: 22px;
  border: 1px solid rgba(121, 240, 255, 0.18);
  border-radius: 22px;
  background:
    linear-gradient(180deg, rgba(8, 18, 30, 0.94), rgba(12, 29, 46, 0.88)),
    radial-gradient(circle at top left, rgba(121, 240, 255, 0.08), transparent 30%);
}

.panel-header,
.panel-grid,
.signal-code,
.capability-grid,
.command-deck,
.architecture-strip {
  display: grid;
}

.panel-header {
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
  color: var(--text-dim);
  font-size: 0.78rem;
}

.panel-grid {
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 14px;
  margin-top: 18px;
}

.metric,
.capability-card,
.command-card,
.architecture-node,
.footer-cta {
  border: 1px solid var(--line);
  background: var(--bg-elevated);
}

.metric {
  padding: 14px;
  border-radius: 16px;
}

.metric strong {
  display: block;
  margin-top: 6px;
  font-size: 1.35rem;
}

.signal-code {
  margin-top: 18px;
  padding: 18px;
  border-radius: 18px;
  background: rgba(4, 10, 18, 0.92);
  color: #b8ffe0;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 0.88rem;
  line-height: 1.7;
  overflow: auto;
}

.section,
.footer-cta {
  margin-top: 24px;
  padding: 28px;
  border-radius: 26px;
  background: rgba(6, 15, 27, 0.72);
}

.section-heading {
  display: flex;
  align-items: end;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 18px;
}

.section-heading h2,
.footer-cta h2 {
  max-width: 22ch;
  font-family: "Chakra Petch", sans-serif;
  font-size: clamp(1.8rem, 3vw, 2.9rem);
  line-height: 1.02;
}

.capability-grid {
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 16px;
}

.capability-card,
.architecture-node {
  padding: 20px;
  border-radius: 18px;
}

.card-kicker,
.command-label,
.node-file,
.metric-label {
  color: var(--accent);
  font-size: 0.76rem;
  font-weight: 700;
}

.capability-card h3,
.architecture-node h3 {
  margin-top: 14px;
  font-size: 1.18rem;
}

.capability-card p,
.architecture-node p {
  margin-top: 10px;
  color: var(--text-dim);
  line-height: 1.65;
}

.chip {
  display: inline-flex;
  margin-top: 14px;
  padding: 7px 10px;
  border-radius: 999px;
  background: rgba(121, 240, 255, 0.08);
  color: var(--text);
  font-size: 0.82rem;
}

.command-deck {
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
}

.command-card {
  padding: 18px;
  border-radius: 18px;
}

.command-card-wide {
  grid-column: 1 / -1;
}

.command-card pre,
.command-card code {
  margin: 0;
  white-space: pre-wrap;
}

.command-card pre {
  overflow: auto;
  margin-top: 12px;
  padding: 16px;
  border-radius: 14px;
  background: rgba(3, 9, 18, 0.94);
  color: #d8fff0;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  line-height: 1.7;
}

.architecture-strip {
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 16px;
}

.footer-cta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
}

@media (max-width: 1080px) {
  .hero-content,
  .capability-grid,
  .architecture-strip {
    grid-template-columns: 1fr 1fr;
  }
}

@media (max-width: 820px) {
  .page-shell {
    width: min(calc(100% - 20px), var(--content-width));
    padding-top: 12px;
  }

  .topbar,
  .hero-content,
  .section,
  .footer-cta {
    padding: 20px;
  }

  .hero-content,
  .capability-grid,
  .command-deck,
  .architecture-strip,
  .footer-cta {
    grid-template-columns: 1fr;
  }

  .footer-cta {
    display: grid;
  }
}

@media (max-width: 560px) {
  h1 {
    max-width: none;
    font-size: clamp(2.4rem, 15vw, 3.5rem);
  }

  .topbar {
    align-items: flex-start;
    flex-direction: column;
  }

  .topbar-actions {
    width: 100%;
    justify-content: space-between;
  }

  .hero-actions,
  .footer-actions {
    display: grid;
    grid-template-columns: 1fr;
  }

  .button {
    width: 100%;
  }
}
```

- [ ] **Step 4: Verify the stylesheet covers the expected modules**

Run: `rg -n "hero-content|capability-grid|command-deck|architecture-strip|language-toggle|@media" assets/css/homepage.css`

Expected: the file contains layout and responsive rules for every homepage section.

- [ ] **Step 5: Commit the homepage styling**

```bash
git add assets/css/homepage.css
git commit -m "feat: style github pages homepage"
```

### Task 4: Implement language switching and translated copy

**Files:**
- Create: `assets/js/homepage.js`
- Modify: `index.html`

- [ ] **Step 1: Write the failing behavior expectation**

Document the interactive behavior:

```text
On first load, visible copy is English.
Clicking 中文 switches all translatable text to Chinese without reloading.
The active language button updates visually.
```

- [ ] **Step 2: Verify the script file does not already exist**

Run: `test -f assets/js/homepage.js; echo $?`

Expected: `1`

- [ ] **Step 3: Create the translation script**

Create `assets/js/homepage.js` with this content:

```js
const translations = {
  en: {
    nav: { overview: "Overview", github: "GitHub" },
    hero: {
      eyebrow: "Protocol. Persistence. Replication.",
      title: "Build Your Own Redis in Rust",
      description:
        "A toy Redis server in Rust that parses RESP, restores from RDB, replicates leader-follower state, supports streams, and queues transactional commands.",
      primaryCta: "View on GitHub",
      secondaryCta: "Quick Start",
    },
    metrics: {
      protocol: "Protocol",
      persistence: "Persistence",
      replication: "Replication",
      streams: "Queues",
    },
    capabilities: {
      eyebrow: "Core capabilities",
      title: "Built around real Redis subsystems",
      protocol: {
        title: "Protocol",
        body: "Parses Redis protocol messages and routes commands through the server runtime.",
      },
      persistence: {
        title: "Persistence",
        body: "Loads database state from RDB files and restores key metadata.",
      },
      replication: {
        title: "Replication",
        body: "Bootstraps replica handshake, full sync, and replicated write flow.",
      },
      streams: {
        title: "Streams and transactions",
        body: "Supports stream operations and queued transactional execution patterns.",
      },
    },
    quickstart: {
      eyebrow: "Run it",
      title: "Start fast, then probe with redis-cli",
      masterLabel: "Master",
      replicaLabel: "Replica",
      probeLabel: "Probe",
    },
    architecture: {
      eyebrow: "Implementation map",
      title: "How the runtime is assembled",
      protocol: {
        title: "Protocol ingestion",
        body: "RESP parsing and request framing for incoming client and replication traffic.",
      },
      command: {
        title: "Command routing",
        body: "Connection handling, command dispatch, and transaction orchestration.",
      },
      storage: {
        title: "State and restore",
        body: "In-memory storage backed by RDB parsing and initial database hydration.",
      },
      replication: {
        title: "Replication flow",
        body: "Replica-side handshake, sync start, and downstream command consumption.",
      },
    },
    footer: {
      eyebrow: "Source and docs",
      title: "Inspect the code, then dive deeper",
      github: "Open GitHub Repository",
      readmeEn: "Read README",
      readmeZh: "Read Chinese README",
    },
  },
  zh: {
    nav: { overview: "概览", github: "GitHub" },
    hero: {
      eyebrow: "协议。持久化。复制。",
      title: "用 Rust 构建自己的 Redis",
      description:
        "一个用 Rust 编写的 toy Redis server，支持 RESP 解析、RDB 恢复、主从复制、Stream，以及事务命令排队执行。",
      primaryCta: "查看 GitHub 仓库",
      secondaryCta: "快速开始",
    },
    metrics: {
      protocol: "协议",
      persistence: "持久化",
      replication: "复制",
      streams: "队列",
    },
    capabilities: {
      eyebrow: "核心能力",
      title: "围绕真实 Redis 子系统构建",
      protocol: {
        title: "协议处理",
        body: "解析 Redis 协议消息，并把命令路由到服务端运行时。",
      },
      persistence: {
        title: "持久化",
        body: "从 RDB 文件加载数据库状态，并恢复键的元数据。",
      },
      replication: {
        title: "主从复制",
        body: "完成 replica 握手、全量同步和写命令复制流程。",
      },
      streams: {
        title: "流与事务",
        body: "支持 Stream 相关操作，以及事务命令排队和执行模式。",
      },
    },
    quickstart: {
      eyebrow: "开始运行",
      title: "先跑起来，再用 redis-cli 探测",
      masterLabel: "主节点",
      replicaLabel: "从节点",
      probeLabel: "探测命令",
    },
    architecture: {
      eyebrow: "实现映射",
      title: "运行时如何拼装起来",
      protocol: {
        title: "协议入口",
        body: "负责 RESP 解析，以及客户端与复制流量的请求分帧。",
      },
      command: {
        title: "命令路由",
        body: "负责连接处理、命令分发，以及事务编排。",
      },
      storage: {
        title: "状态与恢复",
        body: "以内存存储为核心，并由 RDB 解析完成初始数据装载。",
      },
      replication: {
        title: "复制链路",
        body: "负责 replica 侧握手、同步启动和下游命令消费。",
      },
    },
    footer: {
      eyebrow: "源码与文档",
      title: "先看代码，再继续深入",
      github: "打开 GitHub 仓库",
      readmeEn: "查看英文 README",
      readmeZh: "查看中文 README",
    },
  },
};

const LANG_STORAGE_KEY = "redis-rs-homepage-language";

function getValue(source, path) {
  return path.split(".").reduce((acc, key) => acc && acc[key], source);
}

function applyLanguage(lang) {
  const dictionary = translations[lang] || translations.en;
  document.documentElement.lang = lang === "zh" ? "zh-CN" : "en";

  document.querySelectorAll("[data-i18n]").forEach((node) => {
    const value = getValue(dictionary, node.dataset.i18n);
    if (typeof value === "string") {
      node.textContent = value;
    }
  });

  document.querySelectorAll(".lang-btn").forEach((button) => {
    button.classList.toggle("is-active", button.dataset.lang === lang);
    button.setAttribute("aria-pressed", String(button.dataset.lang === lang));
  });

  window.localStorage.setItem(LANG_STORAGE_KEY, lang);
}

function preferredLanguage() {
  const saved = window.localStorage.getItem(LANG_STORAGE_KEY);
  return saved === "zh" ? "zh" : "en";
}

document.addEventListener("DOMContentLoaded", () => {
  applyLanguage(preferredLanguage());

  document.querySelectorAll(".lang-btn").forEach((button) => {
    button.addEventListener("click", () => {
      applyLanguage(button.dataset.lang === "zh" ? "zh" : "en");
    });
  });
});
```

- [ ] **Step 4: Ensure all user-facing strings use translation hooks**

Update any untranslated visible labels in `index.html` so they either:
- use `data-i18n`, or
- are intentionally non-language-specific code/file labels such as `protocol.rs` or `RESP`

For example, this button group should remain:

```html
<div class="language-toggle" role="group" aria-label="Language switcher">
  <button class="lang-btn is-active" type="button" data-lang="en">EN</button>
  <button class="lang-btn" type="button" data-lang="zh">中文</button>
</div>
```

while user-facing copy such as headings and CTA text should be translated with `data-i18n`.

- [ ] **Step 5: Verify translation coverage**

Run: `rg -n "data-i18n" index.html && node -e "const fs=require('fs'); const html=fs.readFileSync('index.html','utf8'); const keys=[...html.matchAll(/data-i18n=\"([^\"]+)\"/g)].map(m=>m[1]); console.log(keys.length)" && sed -n '1,260p' assets/js/homepage.js | sed -n '1,220p'`

Expected:
- `index.html` contains translation hooks across the page
- script defines both `en` and `zh`
- English remains the default fallback

- [ ] **Step 6: Commit the bilingual interaction**

```bash
git add index.html assets/js/homepage.js
git commit -m "feat: add bilingual homepage interactions"
```

### Task 5: Verify the site in the real Pages pipeline

**Files:**
- Verify: `index.html`
- Verify: `assets/css/homepage.css`
- Verify: `assets/js/homepage.js`
- Verify: `_config.yml`
- Verify: `index.md`

- [ ] **Step 1: Build the site locally**

Run: `bundle exec jekyll build`

Expected: build exits successfully and writes the generated site to `_site/`.

- [ ] **Step 2: Verify the generated root page includes the homepage**

Run: `rg -n "Build Your Own Redis in Rust|language-toggle|Built around real Redis subsystems|Open GitHub Repository" _site/index.html`

Expected: generated root page contains the homepage hero, language toggle, capability section, and footer CTA.

- [ ] **Step 3: Verify the overview page still exists**

Run: `test -f _site/overview/index.html && echo ok`

Expected: `ok`

- [ ] **Step 4: Serve the built site and smoke-check it**

Run:

```bash
ruby -run -e httpd _site -p 4000
```

Then in a second shell:

```bash
curl -s http://127.0.0.1:4000/ | rg "Build Your Own Redis in Rust|redis-rs"
curl -s http://127.0.0.1:4000/overview/ | rg "概览|旨在从 0 到 1 实现一个基础的 redis-server 版本"
```

Expected:
- root responds with homepage content
- `/overview/` responds with the previous markdown overview

- [ ] **Step 5: Commit after successful verification**

```bash
git add index.html index.md _config.yml assets/css/homepage.css assets/js/homepage.js .gitignore
git commit -m "feat: launch github pages project homepage"
```
