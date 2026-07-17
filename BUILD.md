# BUILD.md — Ancestral Brain Alpha
## How to build the Mac app on your machine

These are copy-paste commands. Run them in Terminal (Applications → Utilities → Terminal).
Estimated time: 15–30 min first time (downloads Rust + Node). Subsequent builds: ~2 min.

---

### Prerequisites (do once)

**1. Install Homebrew** (skip if you already have it — check with `brew --version`)
```
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```
After it finishes, follow any instructions it prints about adding brew to your PATH.

**2. Install Node.js**
```
brew install node
```

**3. Install Rust**
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
When it asks, press Enter to accept defaults. Then run:
```
source "$HOME/.cargo/env"
```

**4. Install Tauri CLI dependencies**
```
brew install pkg-config
```

---

### Build the app

**5. Go to the app folder**
```
cd ~/ancestralbrain/app
```
(Replace `~/ancestralbrain` with wherever you cloned the repo.)

**6. Install JavaScript dependencies**
```
npm install
```

**7. Build the app** (creates a .dmg in `src-tauri/target/release/bundle/dmg/`)
```
npm run tauri build -- --target aarch64-apple-darwin
```
This takes 5–10 minutes the first time.

**8. Find your .dmg**
```
open src-tauri/target/release/bundle/dmg/
```
A Finder window opens showing `Ancestral Brain_*.dmg`.

---

### Install Ollama (required for the app to work)

**9. Install Ollama**
```
brew install ollama
```

**10. Start Ollama**
```
ollama serve &
```

**11. Pull the AI models** (one-time, ~8GB download total)
```
ollama pull llama3.1:8b
ollama pull nomic-embed-text
```

---

### Upload the .dmg to Hostinger (for distribution)

**12. Run the upload script** (after building)
```
cd ~/ancestralbrain
python3 upload_dmg.py
```

---

### Troubleshooting

- **"command not found: cargo"** → Run `source "$HOME/.cargo/env"` then try again
- **"command not found: brew"** → Follow the PATH instructions Homebrew printed during install
- **Build fails with linker error** → Run `xcode-select --install` and try the build again
- **App won't open ("unidentified developer")** → Right-click the app → Open → Open anyway (unsigned alpha)
