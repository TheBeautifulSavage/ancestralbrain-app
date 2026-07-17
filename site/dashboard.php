<?php
$sessions_dir = __DIR__ . '/data/sessions';
if (!is_dir($sessions_dir)) mkdir($sessions_dir, 0700, true);
session_save_path($sessions_dir);
session_start();

require_once __DIR__ . '/_helpers.php';

$user = current_user();
if (!$user) {
    header('Location: login.php');
    exit;
}

$model = htmlspecialchars($user['selected_model'] ?: '');
$email = htmlspecialchars($user['email']);
?>
<?= page_head('Dashboard') ?>
<style>
  body { padding: 0; }
  .main { max-width: 780px; margin: 0 auto; padding: 32px 20px 60px; }
  nav { padding: 20px; border-bottom: 1px solid var(--border); margin-bottom: 0; }
  .welcome { font-size: 22px; font-weight: 800; margin-bottom: 6px; }
  .welcome-sub { color: var(--gray); font-size: 14px; margin-bottom: 32px; }
  .card-title { font-size: 16px; font-weight: 700; margin-bottom: 16px; color: var(--white); }
  .badge { display: inline-block; background: rgba(245,166,35,0.12); border: 1px solid rgba(245,166,35,0.3); color: var(--amber); font-size: 12px; font-weight: 600; padding: 4px 10px; border-radius: 4px; letter-spacing: .06em; }
  .platform-detected { background: #1a1a1a; border: 1px solid var(--border); border-radius: 6px; padding: 12px 14px; font-size: 14px; color: #ccc; margin-bottom: 14px; }
  .platform-detected strong { color: var(--amber); }
  select { width: 100%; padding: 11px 14px; background: #1a1a1a; border: 1px solid var(--border); border-radius: 6px; color: #fff; font-size: 15px; font-family: inherit; margin-bottom: 12px; appearance: none; -webkit-appearance: none; cursor: pointer; }
  select:focus { outline: none; border-color: var(--amber); }
  .save-btn { display: inline-block; padding: 10px 24px; background: var(--amber); color: #000; font-weight: 700; font-size: 14px; border: none; border-radius: 6px; cursor: pointer; font-family: inherit; }
  .save-btn:hover { background: #e09610; }
  .save-msg { font-size: 13px; color: #5cb85c; margin-left: 12px; display: none; }
  .dl-btn { display: inline-flex; align-items: center; gap: 8px; padding: 14px 24px; background: var(--amber); color: #000; font-weight: 700; font-size: 16px; border-radius: 8px; text-decoration: none; margin-bottom: 12px; transition: background .15s; }
  .dl-btn:hover { background: #e09610; color: #000; }
  .dl-note { font-size: 13px; color: #666; margin-top: 8px; }
  .steps { counter-reset: steps; list-style: none; }
  .steps li { counter-increment: steps; display: flex; gap: 14px; padding: 14px 0; border-bottom: 1px solid var(--border); font-size: 15px; color: #ccc; }
  .steps li:last-child { border-bottom: none; }
  .steps li::before { content: counter(steps); min-width: 28px; height: 28px; background: rgba(245,166,35,0.15); border: 1px solid rgba(245,166,35,0.4); border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 13px; font-weight: 700; color: var(--amber); flex-shrink: 0; }
  .model-note { font-size: 13px; color: #666; margin-top: 8px; }
</style>

<nav>
  <a href="index.html" class="wordmark" style="margin-bottom:0">Ancestral Brain</a>
  <a href="logout.php" style="font-size:14px;color:#666">Log Out</a>
</nav>

<div class="main">
  <div class="welcome">Welcome back, <?= $email ?></div>
  <div class="welcome-sub">Your private AI vault is ready.</div>

  <!-- Model Selection Card -->
  <div class="card">
    <div class="card-title">🤖 AI Model Selection</div>
    <div class="platform-detected" id="platform-info">Detecting your hardware…</div>
    <p class="model-note" style="margin-bottom:14px;color:#888">Recommended model based on your hardware:</p>
    <div style="margin-bottom:16px;padding:12px 14px;background:#0d0d0d;border:1px solid var(--border);border-radius:6px;font-size:14px" id="model-recommend">Detecting…</div>

    <form id="prefs-form">
      <div class="label">Override Model</div>
      <select name="selected_model" id="model-select">
        <option value="llama3.2:1b" <?= $model === 'llama3.2:1b' ? 'selected' : '' ?>>llama3.2:1b — Fastest, lowest RAM (~1 GB)</option>
        <option value="llama3.2:3b" <?= $model === 'llama3.2:3b' ? 'selected' : '' ?>>llama3.2:3b — Balanced speed + quality (~2 GB)</option>
        <option value="llama3.1:8b" <?= $model === 'llama3.1:8b' ? 'selected' : '' ?>>llama3.1:8b — Best quality, needs 8 GB RAM</option>
      </select>
      <p class="model-note">This model was auto-selected for your hardware. You can override it here.</p>
      <br>
      <button type="submit" class="save-btn">Save Preference</button>
      <span class="save-msg" id="save-msg">✓ Saved</span>
    </form>
  </div>

  <!-- Download Card -->
  <div class="card">
    <div class="card-title">⬇️ Download Ancestral Brain</div>
    <div class="platform-detected" id="dl-platform-info">Detecting your OS…</div>
    <a href="#" class="dl-btn" id="dl-primary">↓ Download</a>
    <p class="dl-note" id="dl-note"></p>
    <p class="dl-note" style="margin-top:12px;color:#555">Unsigned alpha — right-click → Open the first time on Mac</p>
    <div style="margin-top:16px">
      <button onclick="document.getElementById('dl-all').style.display=document.getElementById('dl-all').style.display==='none'?'block':'none'"
        style="background:transparent;border:none;color:#555;font-size:13px;cursor:pointer;text-decoration:underline;padding:0">All platforms ↓</button>
      <div id="dl-all" style="display:none;margin-top:14px;font-size:14px">
        <div style="padding:8px 0;border-bottom:1px solid var(--border);display:flex;justify-content:space-between">
          <span style="color:#999">🍎 Mac — Apple Silicon (M1/M2/M3/M4)</span>
          <a href="https://github.com/TheBeautifulSavage/ancestralbrain-app/releases/latest/download/Ancestral.Brain_0.1.0_aarch64.dmg" style="color:var(--amber);font-weight:600">.dmg ↓</a>
        </div>
        <div style="padding:8px 0;border-bottom:1px solid var(--border);display:flex;justify-content:space-between">
          <span style="color:#999">🍎 Mac — Intel</span>
          <a href="https://github.com/TheBeautifulSavage/ancestralbrain-app/releases/latest/download/Ancestral.Brain_0.1.0_x64.dmg" style="color:var(--amber);font-weight:600">.dmg ↓</a>
        </div>
        <div style="padding:8px 0;border-bottom:1px solid var(--border);display:flex;justify-content:space-between">
          <span style="color:#999">🪟 Windows</span>
          <a href="https://github.com/TheBeautifulSavage/ancestralbrain-app/releases/latest/download/Ancestral.Brain_0.1.0_x64_en-US.msi" style="color:var(--amber);font-weight:600">.msi ↓</a>
        </div>
        <div style="padding:8px 0;display:flex;justify-content:space-between">
          <span style="color:#999">🐧 Linux (AppImage)</span>
          <a href="https://github.com/TheBeautifulSavage/ancestralbrain-app/releases/latest/download/ancestral-brain_0.1.0_amd64.AppImage" style="color:var(--amber);font-weight:600">.AppImage ↓</a>
        </div>
      </div>
    </div>
  </div>

  <!-- Install Steps -->
  <div class="card">
    <div class="card-title">🚀 Getting Started</div>
    <ol class="steps">
      <li>Open the installer — drag Ancestral Brain to your Applications folder (Mac) or run the installer (Windows/Linux)</li>
      <li>Launch the app</li>
      <li>The app detects your hardware and downloads the right AI model automatically — one-time download, 2–8 GB depending on your model</li>
      <li>Pick your vault folder — your files never leave your machine</li>
    </ol>
  </div>
</div>

<script>
(function () {
  var BASE = 'https://github.com/TheBeautifulSavage/ancestralbrain-app/releases/latest/download/';

  var ua = navigator.userAgent;
  var pl = navigator.platform;

  var isAppleSilicon = ua.includes('Mac') && (
    ua.includes('arm') ||
    (pl === 'MacIntel' && navigator.maxTouchPoints > 1)
  );
  var isMacIntel = ua.includes('Mac') && !ua.includes('iPhone') && !ua.includes('iPad') && !isAppleSilicon;
  var isWindows   = ua.includes('Win');
  var isLinux     = ua.includes('Linux') && !ua.includes('Android');

  // RAM detection (rough)
  var ram = navigator.deviceMemory || null;
  var ramStr = ram ? ram + ' GB' : 'unknown RAM';

  // Platform display
  var platformName = 'Unknown OS';
  var platformInfo, dlHref, dlLabel, dlNote, recommendedModel, recommendNote;

  if (isAppleSilicon) {
    platformName = 'Mac Apple Silicon · ' + ramStr;
    dlHref = BASE + 'Ancestral.Brain_0.1.0_aarch64.dmg';
    dlLabel = '↓ Download for Mac (Apple Silicon)';
    dlNote = 'macOS 13+ · Right-click → Open the first time';
    if (!ram || ram >= 8) {
      recommendedModel = 'llama3.1:8b';
      recommendNote = 'Best quality — your Apple Silicon has enough RAM';
    } else {
      recommendedModel = 'llama3.2:3b';
      recommendNote = 'Balanced — good fit for your ' + ramStr;
    }
  } else if (isMacIntel) {
    platformName = 'Mac Intel · ' + ramStr;
    dlHref = BASE + 'Ancestral.Brain_0.1.0_x64.dmg';
    dlLabel = '↓ Download for Mac (Intel)';
    dlNote = 'macOS 13+ · Right-click → Open the first time';
    recommendedModel = 'llama3.2:3b';
    recommendNote = 'Good balance for Intel Mac';
  } else if (isWindows) {
    platformName = 'Windows · ' + ramStr;
    dlHref = BASE + 'Ancestral.Brain_0.1.0_x64_en-US.msi';
    dlLabel = '↓ Download for Windows';
    dlNote = 'Windows 10+ · Click "More info" → Run anyway if SmartScreen warns';
    recommendedModel = (ram && ram >= 8) ? 'llama3.1:8b' : 'llama3.2:3b';
    recommendNote = 'Auto-selected for your hardware';
  } else if (isLinux) {
    platformName = 'Linux · ' + ramStr;
    dlHref = BASE + 'ancestral-brain_0.1.0_amd64.AppImage';
    dlLabel = '↓ Download for Linux (AppImage)';
    dlNote = 'chmod +x *.AppImage then run it';
    recommendedModel = (ram && ram >= 8) ? 'llama3.1:8b' : 'llama3.2:3b';
    recommendNote = 'Auto-selected for your hardware';
  } else {
    platformName = 'Unknown OS';
    dlHref = 'https://github.com/TheBeautifulSavage/ancestralbrain-app/releases/latest';
    dlLabel = '↓ All Downloads on GitHub';
    dlNote = 'Releases page has all platforms';
    recommendedModel = 'llama3.2:3b';
    recommendNote = 'Balanced default';
  }

  // Update platform info boxes
  var pi = document.getElementById('platform-info');
  var dpi = document.getElementById('dl-platform-info');
  if (pi)  pi.innerHTML = '<strong>Detected:</strong> ' + platformName;
  if (dpi) dpi.innerHTML = '<strong>Detected:</strong> ' + platformName;

  // Update download button
  var dlBtn = document.getElementById('dl-primary');
  var dlNoteEl = document.getElementById('dl-note');
  if (dlBtn)    { dlBtn.href = dlHref; dlBtn.textContent = dlLabel; }
  if (dlNoteEl) dlNoteEl.textContent = dlNote;

  // Update model recommendation
  var recEl = document.getElementById('model-recommend');
  if (recEl) {
    recEl.innerHTML = '<strong style="color:var(--amber)">' + recommendedModel + '</strong>'
      + ' &mdash; <span style="color:#888">' + recommendNote + '</span>';
  }

  // Pre-select recommended model if user hasn't saved one yet
  var sel = document.getElementById('model-select');
  var savedModel = '<?= $model ?>';
  if (sel && !savedModel) {
    for (var i = 0; i < sel.options.length; i++) {
      if (sel.options[i].value === recommendedModel) {
        sel.selectedIndex = i;
        break;
      }
    }
  }

  // Save preferences via AJAX
  var form = document.getElementById('prefs-form');
  var saveMsg = document.getElementById('save-msg');
  if (form) {
    form.addEventListener('submit', function (e) {
      e.preventDefault();
      var model = document.getElementById('model-select').value;
      fetch('save_prefs.php', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ selected_model: model, platform: platformName })
      })
      .then(function (r) { return r.json(); })
      .then(function (d) {
        if (d.ok && saveMsg) {
          saveMsg.style.display = 'inline';
          setTimeout(function () { saveMsg.style.display = 'none'; }, 2500);
        }
      })
      .catch(function () {});
    });
  }
})();
</script>

</body>
</html>
