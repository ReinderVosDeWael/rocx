"""Build the checkout's experimental layout demo and serve it on localhost."""

import argparse
import os
import socket
import threading
import time
import urllib.request
from functools import partial
from hashlib import sha256
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[1]
EXPERIMENT = ROOT / "javascript/experimental-layout"
FIXTURES = ROOT / "crates/rocx-layout/tests"
FONT_SHA = "ac6e1cd9abc68c06cd7a56ea128d79b55fc0af4bc0be247b9f3737bf01ee11e4"


def stage(dest):
    cli = shutil.which("wasm-bindgen")
    if not cli:
        local_cli = ROOT / "target/release/wasm-bindgen"
        cli = str(local_cli) if local_cli.is_file() else None
    if not cli or subprocess.check_output([cli, "--version"], text=True).strip() != "wasm-bindgen 0.2.130":
        raise SystemExit("Install the matching CLI: cargo install wasm-bindgen-cli --version 0.2.130 --locked")
    font = FIXTURES / "fonts/DejaVuSans-subset.ttf"
    if sha256(font.read_bytes()).hexdigest() != FONT_SHA:
        raise SystemExit("Pinned layout fixture font checksum differs")
    for name in ("index.html", "style.css", "main.mjs", "layout-worker.mjs"):
        shutil.copyfile(EXPERIMENT / "demo" / name, dest / name)
    shutil.copyfile(font, dest / "font.ttf")
    shutil.copyfile(FIXTURES / "fonts/LICENSE.txt", dest / "FONT-LICENSE.txt")
    shutil.copyfile(FIXTURES / "fixtures/physical-plain.docx", dest / "sample.docx")
    with (dest / "font-atlas.json").open("wb") as atlas:
        subprocess.run(
            ["cargo", "run", "--locked", "--quiet", "-p", "rocx-layout",
             "--example", "font_atlas", "--", font],
            cwd=ROOT, stdout=atlas, check=True,
        )
    subprocess.run(
        ["cargo", "build", "--locked", "--release", "--manifest-path", EXPERIMENT / "Cargo.toml",
         "--target", "wasm32-unknown-unknown", "--target-dir", ROOT / "target"],
        cwd=ROOT, check=True,
    )
    subprocess.run(
        [cli, "--target", "web", "--out-dir", dest / "pkg", "--out-name", "rocx",
         ROOT / "target/wasm32-unknown-unknown/release/rocx_js_experimental_layout.wasm"],
        check=True,
    )


def smoke(server):
    """Run Firefox against the staged site; release driver, server and profile."""
    if not shutil.which("geckodriver"):
        raise SystemExit("--smoke requires geckodriver and Firefox")
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        port = sock.getsockname()[1]
    driver_url = f"http://127.0.0.1:{port}"
    # Firefox Snap and geckodriver need a mutually visible profile directory.
    with tempfile.TemporaryDirectory(prefix="rocx-layout-firefox-", dir=Path.home()) as profile:
        args = ["geckodriver", "--port", str(port), "--profile-root", profile, "--log", "error"]
        proc = subprocess.Popen(args)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            deadline = time.monotonic() + 10
            while True:
                if proc.poll() is not None:
                    raise SystemExit("geckodriver exited before accepting connections")
                try:
                    with urllib.request.urlopen(f"{driver_url}/status", timeout=1):
                        break
                except OSError:
                    if time.monotonic() >= deadline:
                        raise SystemExit("geckodriver did not become ready")
                    time.sleep(0.1)
            env = {**os.environ, "PYTHON": sys.executable}
            snap = Path("/snap/firefox/current/usr/lib/firefox/firefox")
            if snap.is_file():
                env.setdefault("FIREFOX_BIN", str(snap.resolve()))
            subprocess.run(
                ["node", EXPERIMENT / "demo/smoke.mjs", f"http://127.0.0.1:{server.server_port}/", driver_url],
                env=env, check=True, timeout=120,
            )
        finally:
            server.shutdown()
            thread.join()
            proc.terminate()
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=8000, help="HTTP port (default: 8000)")
    modes = parser.add_mutually_exclusive_group()
    modes.add_argument("--check", action="store_true", help="Build and verify staging, then exit")
    modes.add_argument("--smoke", action="store_true", help="Verify real Firefox edits and downloads (requires python-docx, Node and geckodriver)")
    args = parser.parse_args()
    preferred = Path("/tmp/opencode")
    with tempfile.TemporaryDirectory(prefix="rocx-layout-demo.", dir=preferred if preferred.is_dir() else None) as tmp:
        dest = Path(tmp)
        stage(dest)
        if args.check:
            print("Layout demo staged successfully.")
            return
        handler = partial(SimpleHTTPRequestHandler, directory=str(dest))
        with ThreadingHTTPServer(("127.0.0.1", 0 if args.smoke else args.port), handler) as server:
            if args.smoke:
                smoke(server)
                return
            print(f"Open http://127.0.0.1:{server.server_port}/ (Ctrl-C to stop)", flush=True)
            server.serve_forever()


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        pass
    except subprocess.CalledProcessError as exc:
        sys.exit(exc.returncode or 1)
