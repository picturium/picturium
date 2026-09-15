<div align="center">
  <a href="https://picturium.cc">
    <img src="public/picturium.png" alt="Picturium" width="420">
  </a>

  <p>Self-hosted, on-the-fly image processing server. Fast, format-agnostic, and built on <a href="https://www.libvips.org/">libvips</a>.</p>

  <p>
    <a href="https://picturium.cc"><strong>picturium.cc</strong></a> ·
    <a href="url_params.md">URL API reference</a> ·
    <a href="config.toml.example">Configuration</a>
  </p>
</div>

---

Picturium sits in front of your images (and documents, and videos) and transforms them on request, via plain URL query parameters — no pre-generated variants, no build step. Point it at a directory, request `photo.jpg?w=800&fit=cover&f=webp`, get back a resized, re-encoded, cached WebP.

## Features

- **Broad input support** — JPEG, PNG, WebP, AVIF, GIF, TIFF, JXL, HEIF, ICO, BMP, JPEG2000, SVG, PDF, EPS/AI/CDR/DXF (via Inkscape), PSD, RAW (CR2, CR3, ARW, DNG, NEF, RAF, ORF, RW2, PEF, NRW, CRW), Office documents (DOC, PPT, XLS, via LibreOffice), and video (MP4, WebM, MKV, MOV, AVI, and more)
- **Modern output formats** — JPEG, PNG, WebP, AVIF, GIF, and JPEG XL, with per-format quality curves and encoder tuning; PDF/SVG passthrough for document sources
- **Full transform pipeline** — resize, crop, aspect ratio, padding, rotation, smart gravity (including attention/entropy-based auto-crop), background fill, DPI, and output quality/size limits with automatic re-encode
- **Animation & video** — frame extraction, clip extraction, frame rate/timing/loop control, and stride sampling, for animated GIF/WebP/AVIF/HEIC sequences and video sources alike
- **Watermarking** — image or text watermarks, positioned, scaled, rotated, and tiled, global or per-request
- **Metadata control** — strip or keep EXIF/ICC/XMP/IPTC/gain-map data per request
- **Built-in caching** — bounded in-memory and on-disk caches (via [foyer](https://github.com/foyer-rs/foyer)) with automatic eviction, plus browser/CDN cache control
- **URL signing** — optional HMAC-SHA256 signature verification to lock down which transforms are allowed
- **Easily configurable** — if the defaults are not good enough, everything is easily configurable using config.toml file or ENV variables

See [`url_params.md`](url_params.md) for the full parameter reference.

## Quick start

Picturium depends on a libvips 8.18.6, which is newer than most package managers provider, so it ships as a Docker image.

```bash
docker pull lamka02sk/picturium

# optional: copy config.toml.example into config.toml and adjust the config
editor config.toml

# run it
docker run --rm -p 20045:20045 \
  -v "$(pwd)/config.toml:/app/config.toml" \
  -v "$(pwd)/data:/app/data" \
  -e PICTURIUM_CONFIG=/app/config.toml \
  lamka02sk/picturium
```

Then request a transform:

```
http://localhost:20045/your-image.jpg?w=800&h=600&fit=cover&f=webp&q=80
```

`data/` is your source directory (served as the URL path root), `cache/` holds the memory/disk cache, and every config key can also be set as `PICTURIUM__SECTION__KEY` — see [`config.toml.example`](config.toml.example) for the full list.

## Development

```bash
./dev.sh
```

Runs the dev container with `cargo watch`, rebuilding and restarting on every change under `src/`.

## Benchmarks

`bench/` compares Picturium against [imgproxy](https://imgproxy.net/) and against the previous Picturium v1 release, on the same files, same scenarios, with output quality scored against a reference render. See [`bench/README.md`](bench/README.md) for how to run it.

## License

[MIT](LICENSE)

## AI disclosure

Picturium code is human-written or reviewed by a human. Documentation and testing handled primarily by AI.
