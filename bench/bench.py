#!/usr/bin/env python3
"""Compare picturium against another engine on speed and output quality.

Engines run in Docker against the same files in ../data. For every scenario the
script measures request latency and throughput, then scores the returned image
against a lanczos3 reference render (SSIM / PSNR) so a smaller file is not
mistaken for a better one.

    python3 bench/bench.py                              # v2 vs imgproxy
    python3 bench/bench.py -e picturium-v2,picturium-v1 # v2 vs v1
"""

import argparse
import statistics
import subprocess
import sys
import time
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

BENCH_DIR = Path(__file__).resolve().parent
OUT_DIR = BENCH_DIR / "out"
COMPOSE = ["docker", "compose", "-f", str(BENCH_DIR / "docker-compose.yml")]

V2 = "http://localhost:20045"
V1 = "http://localhost:20048"
IMGPROXY = "http://localhost:20047"

EXT = {"jpeg": "jpg", "webp": "webp", "avif": "avif", "png": "png", "jxl": "jxl", "gif": "gif"}

# source, box, output format, quality. Quality is given as a number on both
# sides, so neither engine's default quality curve is part of the comparison.
# "only" restricts a scenario to the engines that can serve it at all, and
# "quality": False drops SSIM / PSNR where a libvips reference render is not
# the same picture the servers produce (documents, vectors, animations).
PICTURIUM = ("picturium-v2", "picturium-v1")

SCENARIOS = [
    {"name": "webp 400x300", "src": "1.jpg", "w": 400, "h": 300, "fmt": "webp", "q": 80},
    {"name": "webp 1280x720", "src": "1.jpg", "w": 1280, "h": 720, "fmt": "webp", "q": 80},
    {"name": "webp 1920x1080", "src": "1.jpg", "w": 1920, "h": 1080, "fmt": "webp", "q": 80},
    {"name": "jpeg 800x600", "src": "1.jpg", "w": 800, "h": 600, "fmt": "jpeg", "q": 80},
    {"name": "jpeg 1920x1080", "src": "1.jpg", "w": 1920, "h": 1080, "fmt": "jpeg", "q": 80},
    {"name": "avif 800x600", "src": "1.jpg", "w": 800, "h": 600, "fmt": "avif", "q": 60},
    {"name": "png 800x600 alpha", "src": "transparent.png", "w": 800, "h": 600, "fmt": "png", "q": 80},
    {"name": "webp 800x600 alpha", "src": "transparent.png", "w": 800, "h": 600, "fmt": "webp", "q": 80},
    {"name": "webp 1600x1200 (36 MPix src)", "src": "25.jpg", "w": 1600, "h": 1200, "fmt": "webp", "q": 80},
    {"name": "webp 800x600 (17 MPix src)", "src": "rotate.jpg", "w": 800, "h": 600, "fmt": "webp", "q": 80},
    {"name": "webp 800x600 tiff src", "src": "sample.tiff", "w": 800, "h": 600, "fmt": "webp", "q": 80},
    {"name": "webp 800x600 jp2 src", "src": "sample1.jp2", "w": 800, "h": 600, "fmt": "webp", "q": 80},
    {"name": "webp 800x600 heic src", "src": "subsample/color.heic", "w": 800, "h": 600, "fmt": "webp", "q": 80},
    {"name": "webp 256x256 ico src", "src": "sample.ico", "w": 256, "h": 256, "fmt": "webp", "q": 80},
    {"name": "webp 600x400 gif src still", "src": "sb.gif", "w": 600, "h": 400, "fmt": "webp", "q": 80,
     "v2_extra": "anim=off", "quality": False},
    {"name": "webp 800x600 svg src", "src": "test.svg", "w": 800, "h": 600, "fmt": "webp", "q": 80,
     "quality": False},
    # Documents. Two different costs live here and are worth reading apart:
    # a PDF is rendered by libvips/poppler in-process, while an office format
    # is first converted to PDF by a separate soffice process, which dominates.
    {"name": "doc: pdf 5 MB page 1", "src": "office/manual.pdf", "w": 800, "h": 600, "fmt": "webp", "q": 80,
     "only": PICTURIUM, "quality": False},
    {"name": "doc: pdf 5 MB page 5", "src": "office/manual.pdf", "w": 800, "h": 600, "fmt": "webp", "q": 80,
     "only": PICTURIUM, "quality": False, "v2_extra": "thumb=p:5", "v1_extra": "thumb=p:5"},
    {"name": "doc: pdf 5 MB page 1 jpeg", "src": "office/manual.pdf", "w": 800, "h": 600, "fmt": "jpeg", "q": 80,
     "only": PICTURIUM, "quality": False},
    {"name": "doc: pdf 5 MB thumbnail 200", "src": "office/manual.pdf", "w": 200, "h": 200, "fmt": "webp", "q": 80,
     "only": PICTURIUM, "quality": False},
    {"name": "doc: pdf 408 KB page 1", "src": "vector/sample.pdf", "w": 800, "h": 600, "fmt": "webp", "q": 80,
     "only": PICTURIUM, "quality": False},
    {"name": "doc: docx 641 KB", "src": "office/document.docx", "w": 800, "h": 600, "fmt": "webp", "q": 80,
     "only": PICTURIUM, "quality": False, "requests": 12},
    {"name": "doc: xlsx 4 KB", "src": "office/sheet.xlsx", "w": 800, "h": 600, "fmt": "webp", "q": 80,
     "only": PICTURIUM, "quality": False, "requests": 12},
]


def v2_url(s, quality=True):
    url = f"{V2}/{s['src']}?w={s['w']}&h={s['h']}&fit=cover&g=center&f={s['fmt']}"
    if s.get("v2_extra"):
        url += "&" + s["v2_extra"]
    return f"{url}&q={s['q']}" if quality else url


def v1_url(s, quality=True):
    # v1 resolves the URL path straight off its working directory and always
    # resizes cover / centre, so there is no fit or gravity to pass.
    fmt = "jpg" if s["fmt"] == "jpeg" else s["fmt"]
    url = f"{V1}/data/{s['src']}?w={s['w']}&h={s['h']}&f={fmt}"
    if s.get("v1_extra"):
        url += "&" + s["v1_extra"]
    return f"{url}&q={s['q']}" if quality else url


def imgproxy_url(s, quality=True):
    options = f"rs:fill:{s['w']}:{s['h']}:0/g:ce" + (f"/q:{s['q']}" if quality else "")
    return f"{IMGPROXY}/unsafe/{options}/plain/local:///{s['src']}@{s['fmt']}"


ENGINES = {
    "picturium-v2": (v2_url, f"{V2}/health", 1200),
    "picturium-v1": (v1_url, f"{V1}/data/1.jpg?w=32", 60),
    "imgproxy": (imgproxy_url, f"{IMGPROXY}/health", 60),
}

COMPOSE_SERVICE = {"picturium-v2": "picturium", "picturium-v1": "picturium-v1", "imgproxy": "imgproxy"}


def fetch(url):
    start = time.perf_counter()
    with urllib.request.urlopen(url, timeout=300) as response:
        body = response.read()
    return time.perf_counter() - start, body


def percentile(values, p):
    if not values:
        return float("nan")
    ordered = sorted(values)
    index = min(len(ordered) - 1, int(round((p / 100) * (len(ordered) - 1))))
    return ordered[index]


def attempt(url):
    """One request, reported as (latency, body) or (latency, None) on failure.

    A failed request is data, not a reason to stop: v1 streams its output
    straight off a deterministic cache path, so concurrent requests for the
    same URL overwrite the file another response is still sending and the
    client sees a truncated body. Counting those is the point.
    """
    start = time.perf_counter()
    try:
        elapsed, body = fetch(url)
        # v1 can answer 200 with an empty body when a concurrent request
        # overwrites the file it is streaming, which is a failure, not an image.
        return (elapsed, body) if body else (elapsed, None)
    except Exception:
        return time.perf_counter() - start, None


def measure(url, requests, concurrency):
    with ThreadPoolExecutor(max_workers=concurrency) as pool:
        list(pool.map(lambda _: attempt(url), range(concurrency)))  # warm up
        start = time.perf_counter()
        results = list(pool.map(lambda _: attempt(url), range(requests)))
        wall = time.perf_counter() - start

    bodies = [r[1] for r in results if r[1] is not None]
    if not bodies:
        raise RuntimeError(f"every request to {url} failed")

    # Latency of failed requests is meaningless, so only successes are ranked.
    latencies = [r[0] * 1000 for r in results if r[1] is not None]
    return {
        "p50": statistics.median(latencies),
        "p90": percentile(latencies, 90),
        "p99": percentile(latencies, 99),
        "rps": len(bodies) / wall,
        "bytes": bodies[0],
        "failed": len(results) - len(bodies),
        "total": len(results),
    }


def in_container(*args):
    return subprocess.run(
        COMPOSE + ["exec", "-T", "picturium", "bash", "bench/quality.sh", *args],
        check=True, capture_output=True, text=True,
    ).stdout.strip()


def score(scenario, candidate_path):
    stem = scenario["src"].replace("/", "_")
    reference = f"bench/out/{stem}.{scenario['w']}x{scenario['h']}.ref.png"
    in_container("ref", f"data/{scenario['src']}", str(scenario["w"]), str(scenario["h"]), reference)
    ssim, psnr, size = in_container("score", reference, f"bench/out/{candidate_path.name}").split()
    return ssim, psnr, size


def dimensions(candidate_path):
    try:
        return in_container("dims", f"bench/out/{candidate_path.name}")
    except subprocess.CalledProcessError:
        return "unreadable"


def wait_for(url, timeout):
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            urllib.request.urlopen(url, timeout=10).read()
            return
        except urllib.error.HTTPError:
            return  # the server answered, which is all this probe needs
        except (urllib.error.URLError, OSError):
            time.sleep(2)
    sys.exit(f"{url} did not come up within {timeout}s")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("-n", "--requests", type=int, default=50)
    parser.add_argument("-c", "--concurrency", type=int, default=4)
    parser.add_argument("-e", "--engines", default="picturium-v2,imgproxy",
                        help=f"comma separated, from {', '.join(ENGINES)}")
    parser.add_argument("-k", "--filter", default="", help="only run scenarios whose name contains this")
    parser.add_argument("-o", "--out", default="results.md", help="report file inside bench/")
    parser.add_argument("--no-up", action="store_true", help="use already running servers")
    parser.add_argument("--no-quality", action="store_true", help="latency only, skip SSIM / PSNR")
    parser.add_argument("--default-quality", action="store_true",
                        help="drop the explicit quality, comparing each engine's own default")
    parser.add_argument("--startup-timeout", type=int, default=1200,
                        help="picturium v2 is built from source on first start")
    args = parser.parse_args()

    engines = [e.strip() for e in args.engines.split(",") if e.strip()]
    unknown = [e for e in engines if e not in ENGINES]
    if unknown:
        sys.exit(f"unknown engine(s): {', '.join(unknown)}")

    scenarios = [s for s in SCENARIOS if args.filter in s["name"]]
    if not scenarios:
        sys.exit(f"no scenario matches {args.filter!r}")

    if not args.no_up:
        subprocess.run(COMPOSE + ["up", "-d", *(COMPOSE_SERVICE[e] for e in engines)], check=True)
    print("waiting for servers (picturium v2 builds from source on first run)...", flush=True)
    for engine in engines:
        _, probe, timeout = ENGINES[engine]
        wait_for(probe, args.startup_timeout if timeout > 60 else timeout)

    # quality.sh writes reference renders as root inside the container, so the
    # clean-up has to happen there too.
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    in_container("clean")

    rows = []
    for scenario in scenarios:
        explicit = not args.default_quality
        for engine in engines:
            if engine not in scenario.get("only", tuple(ENGINES)):
                continue
            build_url = ENGINES[engine][0]
            print(f"{scenario['name']:<32} {engine}", flush=True)
            requests = scenario.get("requests", args.requests)
            try:
                result = measure(build_url(scenario, explicit), requests, args.concurrency)
            except (RuntimeError, urllib.error.HTTPError, urllib.error.URLError, OSError) as e:
                print(f"  failed: {e}", flush=True)
                rows.append((scenario["name"], engine, float("nan"), float("nan"), float("nan"),
                             float("nan"), float("nan"), "-", "-", f"error: {e}", "all"))
                continue

            stem = scenario["src"].replace("/", "_")
            path = OUT_DIR / f"{engine}.{stem}.{scenario['w']}x{scenario['h']}.{EXT[scenario['fmt']]}"
            path.write_bytes(result["bytes"])

            ssim = psnr = "-"
            size = dimensions(path)
            if not args.no_quality and scenario.get("quality", True):
                ssim, psnr, size = score(scenario, path)

            failed = f"{result['failed']}/{result['total']}" if result["failed"] else "-"
            if result["failed"]:
                print(f"  {failed} requests failed", flush=True)
            rows.append((scenario["name"], engine, result["p50"], result["p90"], result["p99"],
                         result["rps"], len(result["bytes"]) / 1024, ssim, psnr, size, failed))

    header = ("| scenario | engine | p50 ms | p90 ms | p99 ms | req/s | KiB | SSIM | PSNR dB | size | failed |\n"
              "|---|---|---:|---:|---:|---:|---:|---:|---:|---|---:|")
    lines = [header]
    for name, engine, p50, p90, p99, rps, kib, ssim, psnr, size, failed in rows:
        lines.append(f"| {name} | {engine} | {p50:.1f} | {p90:.1f} | {p99:.1f} | "
                     f"{rps:.1f} | {kib:.1f} | {ssim} | {psnr} | {size} | {failed} |")

    table = "\n".join(lines)
    quality_note = "each engine's default quality" if args.default_quality else "an explicit quality per scenario"
    note = (f"\n{args.requests} requests at concurrency {args.concurrency}, {quality_note}, "
            "SSIM / PSNR against a lanczos3 reference render of the same box. "
            "`failed` counts requests that did not return a complete body; latency and "
            "req/s cover the successful ones only.\n")
    print("\n" + table + note)
    (BENCH_DIR / args.out).write_text(table + note)
    print(f"written to {BENCH_DIR / args.out}")


if __name__ == "__main__":
    main()
