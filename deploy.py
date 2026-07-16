#!/usr/bin/env python3
"""
deploy.py — Ancestral Brain → Hostinger
Usage: python3 deploy.py [file1 file2 ...]  (default: all site/ files)

NOTE: Hostinger credentials endpoint rate-limits aggressively.
Creds are cached to .deploy_creds.json for reuse within their validity window (~6h).
"""
import requests, time, sys, json
from pathlib import Path

TOKEN    = "bOenMo7QIzbYkA6MW0JsURBwqsG1xKP3I6KBLubo0b08d769"
USERNAME = "u411916236"
DOMAIN   = "ancestralbrain.com"
SITE_DIR = Path(__file__).parent / "site"
CREDS_CACHE = Path(__file__).parent / ".deploy_creds.json"

def get_creds():
    import time as _time
    # Use cached creds if fresh (JWT valid ~6h; cache for 5h to be safe)
    if CREDS_CACHE.exists():
        cached = json.loads(CREDS_CACHE.read_text())
        if _time.time() < cached.get("expires_at", 0):
            print("  (using cached creds)")
            return cached["url"], cached["auth_key"], cached["rest_auth_key"]

    r = requests.post(
        "https://developers.hostinger.com/api/hosting/v1/files/upload-urls",
        headers={"Authorization": f"Bearer {TOKEN}", "Content-Type": "application/json"},
        json={"username": USERNAME, "domain": DOMAIN},
        timeout=60,
    )
    if r.status_code != 200:
        print(f"FAILED to get creds: HTTP {r.status_code} — {r.text[:300]}")
        sys.exit(1)
    c = r.json()
    # Cache with 5-hour expiry
    c["expires_at"] = _time.time() + 3600 * 5
    CREDS_CACHE.write_text(json.dumps(c))
    return c["url"], c["auth_key"], c["rest_auth_key"]

def upload(tus_base, auth_key, rest_key, local_path, remote_name):
    size = local_path.stat().st_size
    url  = f"{tus_base.rstrip('/')}/{remote_name}?override=true"
    h = {"X-Auth": auth_key, "X-Auth-Rest": rest_key,
         "upload-length": str(size), "upload-offset": "0"}
    requests.post(url, data="", headers=h, timeout=60)
    patch_h = {**h, "Content-Type": "application/offset+octet-stream", "Tus-Resumable": "1.0.0"}
    with open(local_path, "rb") as f:
        r2 = requests.patch(url, data=f, headers=patch_h, timeout=120)
    ok = r2.status_code in (200, 201, 204)
    print(f"  {'✓' if ok else '✗'} {remote_name} ({size:,}b) — {r2.status_code}")
    if not ok:
        print(f"    {r2.text[:200]}")
    return ok

def main():
    print(f"=== Ancestral Brain Deploy → {DOMAIN} ===\n")
    tus_base, auth_key, rest_key = get_creds()
    print(f"TUS: {tus_base}\n")

    if len(sys.argv) > 1:
        files = [Path(f) for f in sys.argv[1:]]
    else:
        files = sorted(SITE_DIR.iterdir())

    ok = err = 0
    for path in files:
        if not path.is_file():
            continue
        success = upload(tus_base, auth_key, rest_key, path, path.name)
        ok += success; err += (not success)
        time.sleep(0.3)

    print(f"\n{'✅ All deployed' if err == 0 else f'⚠ {ok} ok / {err} failed'}")
    print(f"Live: https://{DOMAIN}/")

if __name__ == "__main__":
    main()
