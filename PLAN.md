# PLAN.md — Ancestral Brain

## Stack (locked — do not relitigate)
- Site: static HTML/CSS/vanilla JS, no framework, no build step → Hostinger
- Waitlist: PHP endpoint → SQLite at ../private/ (outside public_html)
- Desktop app: Tauri (Rust + web frontend). Mac/Apple Silicon first.
- Local AI: Ollama at localhost:11434. Never bundled.
- Index: SQLite + sqlite-vec. Single file in app data dir.
- Secrets: env vars only, .env gitignored
- Repo: single git repo, site/ + app/ dirs. Commit after every completed task.

---

## PHASE 0 — Setup (HOURS)
- [x] Init git repo (main branch, site/ + app/ dirs)
- [x] .gitignore (.env, node_modules, target/, *.sqlite, *.db)
- [x] PROGRESS.md, BACKLOG.md, PLAN.md
- [ ] Test Hostinger deploy: upload hello.html → verify at ancestralbrain.com → delete
- [ ] 🧑 HUMAN: confirm DNS points to Hostinger (check now — domain already on account)

## PHASE 1 — Landing Page + Waitlist LIVE (DAYS)
Deliverable: ancestralbrain.com live with:
- Hero: "Ancestral Brain — your life's work, running at home"
- 3 benefit blocks: Private/local · Ask your archive anything · Built for producers first
- 60-sec explainer section (placeholder for video embed — not blocking)
- Email waitlist form (name optional, email required)
- Footer: privacy note "we store your email, nothing else"

Constraints:
- Dark, minimal, one accent color
- Fast: <100KB page weight
- No web fonts beyond one family
- Mobile-first

- [x] waitlist.php (PHP → SQLite, CORS-locked to domain, validates input)
- [ ] index.html (Haiku subagent — in progress)
- [ ] Deploy both to Hostinger
- [ ] Submit test email → verify stored → form rejects garbage
- [ ] Verify on mobile viewport
- [ ] 🧑 HUMAN (optional): Stripe $99 refundable founder deposit link → add button if provided
- [ ] 🧑 HUMAN (later): record explainer video, swap placeholder
- [ ] Commit: "phase 1: landing page + waitlist live"

## PHASE 2 — Producer Vault MVP (6–8 WEEKS)
Build in strict order:

### 2a. Skeleton
- Tauri app init (Mac/Apple Silicon)
- Onboarding screen: pick folders to index
- Ollama detection → guided install screen if absent

### 2b. Indexer
- Walk chosen folders; for each file: path, type, size, dates, audio metadata
- Audio metadata: format, duration, sample rate, BPM/key if cheap via existing crates
- Text-bearing content (docs, PDFs): chunk + embed → SQLite+sqlite-vec
- Resumable + re-runnable (skip unchanged files by mtime+size)
- Progress UI
- Acceptance: index 500 mixed files without crash; re-run = 0 files indexed

### 2c. Search/Chat
- One screen: search box
- Results: semantic + filename match, merged
- Chat mode: RAG over index via Ollama, sources as clickable file paths (reveal in Finder)
- Acceptance: "find my voice memos about the bridge section" returns planted test file

### 2d. Voice Memo Transcription
- whisper.cpp small model for audio transcription
- Transcripts + filename/folder/metadata all go into index
- Acceptance: planted audio memo findable by spoken content

### 2e. Polish
- App icon, empty states, error states (Ollama down, folder missing)
- Settings screen: folders, model choice
- Nothing else

### 2f. Packaging
- Build signed + notarized .dmg
- 🧑 HUMAN: Apple Developer account ($99/yr) + signing creds required → print exact steps, wait

### 2g. Gate + Ship
- Free tier: 1 folder, 5k files
- Paid: unlimited
- 🧑 HUMAN: choose licensing/payments provider → integrate simplest key check (offline-tolerant)
- Do NOT build own licensing server
- Update website: replace waitlist CTA with download + pricing

## PHASE 3 — DO NOT START
(Family mode, Windows, hardware, audio-similarity search → BACKLOG)

---

## Architecture Decisions (locked)
- Audio similarity/sonic search: BACKLOG — not v1
- whisper.cpp small model for voice memo transcription
- No bundled AI models — Ollama only
- Repo: commit after every task, one-line message
