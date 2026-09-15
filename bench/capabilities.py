#!/usr/bin/env python3
"""Probe which requests picturium v1 and v2 can actually serve.

Latency only tells half the upgrade story: a lot of what v2 added is requests
v1 answers with an error or not at all. Each probe below is one URL per engine,
reported as the status code and response size, so a capability that exists on
paper but 500s shows up as a failure rather than a feature.

    python3 bench/capabilities.py
"""

import argparse
import time
import urllib.error
import urllib.request
from pathlib import Path

BENCH_DIR = Path(__file__).resolve().parent
V2 = "http://localhost:20045"
V1 = "http://localhost:20048"

# name, source, v2 query, v1 query. A v1 query of None means v1 has no
# equivalent parameter at all, rather than having one that fails.
#
# v1 answers a source it cannot decode with the original file's bytes rather
# than an error, so a 200 alone does not mean it processed anything. Every
# probe therefore asks for a concrete output format and the returned
# Content-Type is checked against it.
PROBES = [
    # --- inputs v1 never supported
    ("input: JXL", "cog.jxl", "w=400&f=webp", "w=400&f=webp"),
    ("input: RAW (Sony ARW)", "raw/sample.arw", "w=400&f=webp", "w=400&f=webp"),
    ("input: RAW (Canon CR3)", "raw/sample.cr3", "w=400&f=webp", "w=400&f=webp"),
    ("input: RAW (Nikon NEF)", "raw/sample.nef", "w=400&f=webp", "w=400&f=webp"),
    ("input: PSD", "vector/sample.psd", "w=400&f=webp", "w=400&f=webp"),
    ("input: EPS", "vector/sample.eps", "w=400&f=webp", "w=400&f=webp"),
    ("input: AI", "vector/sample.ai", "w=400&f=webp", "w=400&f=webp"),
    ("input: CDR", "vector/sample.cdr", "w=400&f=webp", "w=400&f=webp"),
    ("input: DXF", "vector/sample.dxf", "w=400&f=webp", "w=400&f=webp"),
    ("input: video MP4 still", "video/video.mp4", "w=400&f=webp", "w=400&f=webp"),
    ("input: video MKV still", "video/video.mkv", "w=400&f=webp", "w=400&f=webp"),
    ("input: video WEBM still", "video/video.webm", "w=400&f=webp", "w=400&f=webp"),
    ("input: BMP", "sample.bmp", "w=400&f=webp", "w=400&f=webp"),
    ("input: ICO", "sample.ico", "w=256&f=webp", "w=256&f=webp"),
    ("input: HEIC", "subsample/color.heic", "w=400&f=webp", "w=400&f=webp"),
    ("input: JP2", "sample1.jp2", "w=400&f=webp", "w=400&f=webp"),
    ("input: TIFF", "sample.tiff", "w=400&f=webp", "w=400&f=webp"),

    # --- outputs
    ("output: JXL", "1.jpg", "w=400&f=jxl", "w=400&f=jxl"),
    ("output: GIF", "1.jpg", "w=400&f=gif", "w=400&f=gif"),
    ("output: AVIF", "1.jpg", "w=400&f=avif&q=60", "w=400&f=avif&q=60"),
    ("output: PDF from SVG", "test.svg", "f=pdf", "f=pdf"),
    ("output: animated WEBP from GIF", "sb.gif", "w=300&f=webp", "w=300&f=webp"),
    ("output: animated GIF from GIF", "sb.gif", "w=300&f=gif", "w=300&f=gif"),
    ("output: animated WEBP from video", "video/video.mp4",
     "w=300&f=webp&anim=frames:12|fps:12", "w=300&f=webp"),

    # --- geometry
    ("geometry: fit=contain + bg", "1.jpg", "w=400&h=400&fit=contain&bg=red", None),
    ("geometry: fit=force", "1.jpg", "w=400&h=400&fit=force", None),
    ("geometry: aspect ratio", "1.jpg", "w=400&ar=16/9", None),
    ("geometry: gravity=attention", "1.jpg", "w=400&h=400&g=attention", None),
    ("geometry: gravity=entropy", "1.jpg", "w=400&h=400&g=entropy", None),
    ("geometry: gravity=top-left", "1.jpg", "w=400&h=400&g=top-left", None),
    ("geometry: padding", "1.jpg", "w=400&pad=20,40&bg=black", None),
    ("geometry: scale", "1.jpg", "scale=0.25", None),
    ("geometry: dpr", "1.jpg", "w=400&dpr=2", "w=400&dpr=2"),
    ("geometry: upsize", "waterfall_400.jpg", "w=1200&upsize=true", None),
    ("geometry: crop", "1.jpg", "w=400&crop=ar:1/1", "w=400&crop=1:1"),
    ("geometry: extend=mirror", "1.jpg", "w=400&h=400&fit=contain&extend=mirror", None),
    ("geometry: resample=nearest", "1.jpg", "w=400&resample=nearest", None),
    ("geometry: rotate 90", "1.jpg", "w=400&rot=90", "w=400&rot=right"),
    ("geometry: rotate 180", "1.jpg", "w=400&rot=180", "w=400&rot=upside-down"),

    # --- filters and watermark
    ("filter: blur", "1.jpg", "w=400&filter=blur:5", None),
    ("filter: sharpen", "1.jpg", "w=400&filter=sharpen:3", None),
    ("filter: grayscale", "1.jpg", "w=400&filter=grayscale:1", None),
    ("filter: sepia", "1.jpg", "w=400&filter=sepia:1", None),
    ("filter: chained", "1.jpg", "w=400&filter=brightness:1.2|contrast:1.1|saturate:0.8", None),
    ("filter: duotone palette", "1.jpg", "w=400&filter=palette:navy,gold", None),
    ("filter: pixelate", "1.jpg", "w=400&filter=pixelate:8", None),
    ("watermark: text", "1.jpg", "w=600&watermark=text:picturium|opacity:60", None),
    ("watermark: image", "1.jpg", "w=600&watermark=image:colors.png|anchor:center", None),

    # --- output control
    ("output: byte size cap", "1.jpg", "w=1200&limit=size:50K", None),
    ("output: dimension cap", "1.jpg", "w=9000&limit=dimension:800", None),
    ("output: quality level word", "1.jpg", "w=400&f=webp&q=high", None),
    ("output: keep ICC metadata", "1.jpg", "w=400&f=jpeg&metadata=icc", None),
    ("output: named colour bg", "1.jpg", "w=400&h=400&fit=contain&bg=rebeccapurple", None),
    ("output: hwb colour bg", "1.jpg", "w=400&h=400&fit=contain&bg=hwb(210%2020%25%2010%25)", None),
    ("output: force regenerate", "1.jpg", "w=400&force=true", None),

    # --- documents
    ("document: PDF page 1", "office/manual.pdf", "w=400&f=webp", "w=400&f=webp"),
    ("document: PDF page 5", "office/manual.pdf", "w=400&f=webp&thumb=p:5", "w=400&f=webp&thumb=p:5"),
    ("document: DOCX", "office/document.docx", "w=400&f=webp", "w=400&f=webp"),
    ("document: XLSX", "office/sheet.xlsx", "w=400&f=webp", "w=400&f=webp"),
]


EXPECTED_TYPE = {
    "webp": "image/webp",
    "jpeg": "image/jpeg",
    "jpg": "image/jpeg",
    "png": "image/png",
    "avif": "image/avif",
    "jxl": "image/jxl",
    "gif": "image/gif",
    "pdf": "application/pdf",
}


def requested_format(query):
    for part in query.split("&"):
        if part.startswith("f="):
            return part[2:]
    return None


class Result:
    def __init__(self, status, size, content_type, processed):
        self.status = status
        self.size = size
        self.content_type = content_type
        self.processed = processed

    @property
    def missing(self):
        return self.status == "n/a"


def probe(base, prefix, src, query, timeout):
    if query is None:
        return Result("n/a", 0, "", False)

    url = f"{base}/{prefix}{src}?{query}"
    try:
        with urllib.request.urlopen(url, timeout=timeout) as response:
            body = response.read()
            content_type = response.headers.get("Content-Type", "").split(";")[0]
        wanted = EXPECTED_TYPE.get(requested_format(query) or "")
        # A 200 whose type is not the type that was asked for is v1 handing
        # back the source file untouched, which is not the capability.
        processed = bool(body) and (wanted is None or content_type == wanted)
        return Result(str(response.status), len(body), content_type, processed)
    except urllib.error.HTTPError as e:
        return Result(str(e.code), 0, "", False)
    except (urllib.error.URLError, OSError) as e:
        return Result(f"err {e}", 0, "", False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("-t", "--timeout", type=int, default=180)
    parser.add_argument("-k", "--filter", default="")
    parser.add_argument("-o", "--out", default="capabilities.md")
    args = parser.parse_args()

    rows = []
    for name, src, v2_query, v1_query in PROBES:
        if args.filter not in name:
            continue
        v2 = probe(V2, "", src, v2_query, args.timeout)
        v1 = probe(V1, "data/", src, v1_query, args.timeout)
        print(f"{name:<40} v2 {v2.status:>6} {v2.size:>9}B {v2.content_type:<16}"
              f"v1 {v1.status:>6} {v1.size:>9}B {v1.content_type}", flush=True)
        rows.append((name, src, v2, v1))

    def verdict(v2, v1):
        if v2.processed and v1.processed:
            return "both"
        if v2.processed:
            return "**v2 only**"
        if v1.processed:
            return "**v1 only**"
        return "neither"

    def cell(r):
        if r.missing:
            return "n/a"
        if not r.processed:
            # Name why it is not a capability: an error, or the source echoed back.
            return f"{r.status} passthrough {r.content_type}" if r.size else r.status
        return f"{r.status} · {r.size / 1024:.1f} KiB"

    lines = ["| capability | source | v2 | v1 | |",
             "|---|---|---|---|---|"]
    for name, src, v2, v1 in rows:
        lines.append(f"| {name} | `{src}` | {cell(v2)} | {cell(v1)} | {verdict(v2, v1)} |")

    table = "\n".join(lines)
    print("\n" + table)
    (BENCH_DIR / args.out).write_text(
        table + "\n\n`n/a` means the engine has no equivalent parameter at all, rather than "
                "one that fails. `passthrough` is a 200 that returned the source file "
                "untouched instead of the requested format, which v1 does for every input "
                "it cannot decode. Sizes are the first response, not a median.\n")
    print(f"\nwritten to {BENCH_DIR / args.out}")


if __name__ == "__main__":
    main()
