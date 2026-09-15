# picturium benchmarks

Two comparisons live here: picturium v2 against imgproxy (below) and v2 against
the released v1 image (`results-v1-v2.md`, plus the capability matrix in
`capabilities.md`).

```bash
python3 bench/bench.py -e picturium-v2,picturium-v1 -n 50 -c 4 -o results-v1-v2-c4.md
python3 bench/capabilities.py
```

`-e` picks the engines to run, from `picturium-v2`, `picturium-v1` and
`imgproxy`; `-o` names the report file inside `bench/`. The v1 service needs the
host's `/usr/share/fonts` mounted (the compose file does this) or LibreOffice
will not start.

## picturium vs imgproxy

Runs both servers in Docker against the same files in `../data`, measures
request latency and throughput per scenario, and scores each returned image
against a reference render so a smaller file is not read as a better one.

## Run

```bash
python3 bench/bench.py
```

```bash
python3 bench/bench.py -n 200 -c 8 -k webp
```

Options: `-n` requests per scenario, `-c` concurrency, `-k` substring filter on
the scenario name, `--no-up` to use servers that are already running,
`--no-quality` for latency only, `--default-quality` to drop the explicit
quality and compare each engine's own default instead. Results are printed and written to
`bench/results.md`.

The first run builds picturium in release mode inside the dev image, which takes
minutes; `docker compose down` when finished.

`BENCH_CPUS` (default 4) caps the CPUs each container gets, and
`IMGPROXY_IMAGE` overrides the imgproxy image.

## What is measured

Each scenario is one URL per engine, requesting the same box, crop, output
format and an explicit numeric quality, so neither engine's default quality
curve is part of the comparison:

| | picturium | imgproxy |
|---|---|---|
| resize | `w`, `h`, `fit=cover` | `rs:fill:w:h:0` |
| crop | `g=center` | `g:ce` |
| format | `f=webp` | `@webp` |
| quality | `q=80` | `q:80` |

Picturium runs with both caches off (`bench/picturium.toml`), so every request
pays the full decode, process and encode cost, which is what imgproxy does.
Metadata is stripped on both sides, and both are limited to the same CPU count.

`SSIM` and `PSNR` compare the returned image to a lanczos3 reference render of
the same box (`vips thumbnail --crop centre`, lossless PNG). Read them together
with the `KiB` column: an engine can always buy fidelity with bytes.

## Default quality

Under `--default-quality` the two engines are not at the same operating point.
imgproxy uses one number per format whatever the size (webp 79, jpeg 80,
avif 63, PNG lossless), while picturium interpolates between the `max` and
`min` of its `[output.quality_curves]` table across 0.25 to 8 MPix, so quality
falls as the output grows. Effective quality measured by matching the default
response byte for byte against explicit `q` values:

| output | picturium | imgproxy |
|---|---:|---:|
| webp 0.12 MPix | 78 | 79 |
| webp 0.92 MPix | 74 | 79 |
| webp 1.92 MPix | 69 | 79 |
| jpeg 0.48 MPix | 78 | 80 |
| avif 0.48 MPix | 66 | 63 |

So the defaults agree on small images and diverge on large ones, where
picturium spends fewer bytes than imgproxy for the same box.

## Caveats

- The reference is a libvips render, and both engines use libvips, so this
  measures fidelity of the resize and encode path, not which output looks
  better to a person. For a source of truth on visual quality, look at the
  files in `bench/out/`.
- Encoder settings, not the code, drive most of the difference: picturium's
  `[output.encoder]` table (webp `smart_subsample`, avif `effort`, ...) against
  imgproxy's `IMGPROXY_*` defaults. Change either side and the numbers move.
- PNG below quality 100 is palette quantization in picturium, while imgproxy
  writes lossless PNG, so that row compares two different things on purpose:
  much smaller against pixel exact.
- Latency here includes decoding the source on every request. With large
  sources that dominates, and the differences between the engines shrink.
