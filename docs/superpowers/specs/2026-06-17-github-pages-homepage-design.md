# GitHub Pages Homepage Design for `redis-rs`

## Goal

Build a distinctive project homepage for `redis-rs` using GitHub Pages.

The homepage should:

- present the project as a polished engineering artifact rather than a plain README mirror
- default to English
- support in-page English/Chinese switching
- link clearly to the GitHub repository
- remain simple to deploy and maintain inside this Rust repository

## Scope

This work covers a single static homepage deployed from the repository itself.

In scope:

- one-page project homepage under `docs/`
- bilingual copy for English and Chinese
- English as the initial language
- responsive layout for desktop and narrow screens
- GitHub repository CTA and README entry points
- lightweight interaction and motion

Out of scope:

- blogs, changelog pages, or multi-page docs IA
- external CMS or site generator adoption
- build tooling such as Vite, React, or bundlers
- server-side rendering or dynamic backend integration

## Recommended Direction

Use the approved `A. Signal Wall` direction.

This direction gives the repo a stronger first screen than a documentation-first layout while still preserving the practical information a visitor needs:

- what the project is
- what it supports
- how to run it
- how the implementation is structured
- where the source lives

It is the best fit for a GitHub-hosted project page because it balances visual identity with engineering clarity.

## Audience

Primary audiences:

- developers browsing the repository from GitHub
- Rust learners interested in implementing Redis concepts
- people evaluating the project quickly from a shared link

What they need from the homepage:

- a fast explanation of what `redis-rs` does
- confidence that the project has real implemented subsystems
- a short path to source code and setup commands

## Content Strategy

The page should be grounded in the current repository, not generic Redis marketing.

Core content signals should come from the current README and source structure:

- RESP protocol handling
- basic Redis commands
- RDB parsing and initialization
- leader-follower replication
- streams
- transaction queueing and execution

The architecture strip should map visible source modules into readable concepts:

- `protocol.rs`
- `server.rs`
- `cmd.rs`
- `storage.rs`
- `rdb.rs`
- `replication_client.rs`

## Information Architecture

The homepage should be a single scrolling page in this order:

1. Hero
2. Capability modules
3. Quick start
4. Architecture strip
5. Repository and docs CTA

### 1. Hero

Purpose:

- establish the project's identity immediately
- communicate the core value in one screen
- expose the language switch and main calls to action

Content:

- project name: `redis-rs`
- English headline: `Build Your Own Redis in Rust`
- Chinese equivalent for toggle mode
- short supporting sentence summarizing protocol, persistence, replication, streams, and transactions
- primary CTA: GitHub repository
- secondary CTA: jump to Quick Start
- language toggle: `EN` and `中文`

### 2. Capability Modules

Purpose:

- communicate real technical depth quickly

Presentation:

- four visually strong modules rather than generic feature cards

Modules:

- Protocol
- Persistence
- Replication
- Streams and Transactions

Each module should have:

- short title
- concise explanatory copy
- one supporting technical cue, such as a protocol or command label

### 3. Quick Start

Purpose:

- make the project feel runnable, not just descriptive

Content:

- `cargo run` startup examples for master and replica roles
- selected `redis-cli` commands drawn from the README
- short section labels that also translate during language switching

This section should look like an integrated command deck, not a plain markdown code dump.

### 4. Architecture Strip

Purpose:

- show that the implementation has understandable internal structure

Presentation:

- a horizontal or stacked system view depending on viewport width
- each segment maps one or more source files to a subsystem responsibility

Example conceptual mapping:

- protocol ingestion
- command routing
- in-memory storage
- RDB loading
- replication flow

This is not a formal diagram tool output. It is a stylized structural band embedded in the page.

### 5. Repository and Docs CTA

Purpose:

- give a clean exit path into the real repo and supporting docs

Content:

- GitHub repo link
- README link
- Chinese README link

This section should be visually simpler than the hero and close the page cleanly.

## Visual Design

Approved aesthetic: dark cyber technical.

The page should feel like a polished systems project page, not a startup landing page.

### Visual principles

- dark blue-black foundation
- cyan and mint accents
- restrained glow, grid, scanline, and signal motifs
- sharp, structured layout with clear visual hierarchy
- command-line and systems cues without becoming cluttered

### Typography

- distinctive display face for hero headline
- high-legibility sans-serif or equivalent for body copy
- monospace for code and protocol cues

### Motion

Use only lightweight effects:

- hero entrance
- hover states
- subtle flowing accents in system bands
- smooth language switch transitions

Avoid:

- heavy 3D scenes
- scroll-jacking
- large scripted animation systems

## Interaction Design

The only meaningful stateful interaction is language switching.

Requirements:

- default language is English
- switching to Chinese updates the whole page content in place
- switching does not reload the page
- labels, buttons, section copy, and navigation text all update together
- active language state is visually clear

Optional enhancement:

- remember the user's last language via local storage if implemented simply

If persistence adds unnecessary complexity, it may be skipped.

## Technical Design

Implement as a static GitHub Pages-friendly site by reusing the repository's existing Jekyll Pages pipeline.

Planned files:

- `index.html`
- `assets/css/homepage.css`
- `assets/js/homepage.js`

Optional supporting files:

- `assets/images/*` for light textures or decorative bitmap assets if needed

Rationale:

- the repository already ships a root-level Jekyll Pages workflow, so reusing it is lower risk than replacing it
- no extra frontend build pipeline required
- minimal maintenance burden
- easy for future contributors to edit

Implementation note:

- preserve the existing markdown overview by moving it off the root route
- serve the new homepage from the root Pages entrypoint

## Content Model

Use a small structured translation object in `docs/app.js` keyed by language.

The script should:

- hold English and Chinese copy
- update text targets through `data-i18n` attributes
- update toggle button active state
- optionally persist the selected language

This avoids duplicating page markup for each language and keeps all copy synchronized.

## Responsive Behavior

Requirements:

- hero remains readable and visually strong on mobile
- command blocks do not overflow horizontally beyond normal code scrolling behavior
- architecture strip collapses into stacked sections on narrow screens
- CTA group and language switch remain accessible on small screens

The design should prioritize a stable layout over aggressive scaling tricks.

## Accessibility and UX Constraints

- maintain sufficient text contrast in all sections
- use semantic headings and buttons/links
- ensure language toggle is keyboard reachable
- preserve readability even with decorative effects enabled
- avoid motion that obscures content or distracts from scanning

## Verification Plan

Before claiming completion:

1. Serve the static files locally and load the page in a browser.
2. Verify English is the default language on first load.
3. Verify switching to Chinese updates visible copy correctly.
4. Verify GitHub and README links resolve correctly.
5. Verify desktop and narrow-screen layouts remain intact.

## Implementation Notes

- Keep edits scoped to the new `docs/` page assets and any minimal supporting repo configuration.
- Do not replace or heavily rewrite the existing README files.
- Do not introduce a framework or static site generator for this task.

## Risks and Mitigations

### Risk: homepage becomes a README clone

Mitigation:

- keep sections concise and visually staged
- use README as source material, not as page structure

### Risk: bilingual support becomes brittle

Mitigation:

- centralize copy in one translation object
- bind text updates through explicit selectors or `data-i18n`

### Risk: visual effects hurt readability

Mitigation:

- keep background effects low contrast
- keep body text and command zones visually calm

## Acceptance Criteria

The work is complete when:

- a GitHub Pages-compatible homepage exists under `docs/`
- the page defaults to English
- the page supports in-place English/Chinese switching
- the page includes a visible GitHub repository link
- the page reflects the real project capabilities and module structure
- the page works on desktop and narrow screens without layout breakage
