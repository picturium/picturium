#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" != "--inside" ]]; then
    repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    exec docker run --rm \
        -v "${repo_root}:/root/picturium" \
        -v "${repo_root}/../picturium-libvips:/root/picturium-libvips" \
        -w /root/picturium \
        lamka02sk/picturium-dev:8.18.4 \
        bash tests/bg_pipeline.sh --inside
fi

work_dir="$(mktemp -d /tmp/picturium-bg-pipeline.XXXXXX)"
server_pid=""

cleanup() {
    local status=$?
    if [[ "${status}" -ne 0 && -f "${work_dir}/server.log" ]]; then
        tail -n 120 "${work_dir}/server.log"
    fi
    if [[ -n "${server_pid}" ]]; then
        kill "${server_pid}" 2>/dev/null || true
        wait "${server_pid}" 2>/dev/null || true
    fi
    rm -rf "${work_dir}"
    return "${status}"
}
trap cleanup EXIT

mkdir -p "${work_dir}/data" "${work_dir}/cache"

printf '%s\n' \
    '<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20">' \
    '<rect width="40" height="20" fill="#0000ff"/>' \
    '</svg>' > "${work_dir}/data/source.svg"
vips copy "${work_dir}/data/source.svg" "${work_dir}/data/source.png"

printf '%s\n' \
    '<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10">' \
    '<rect width="10" height="10" fill="rgba(0,0,0,0)"/>' \
    '</svg>' > "${work_dir}/data/transparent.svg"
vips copy "${work_dir}/data/transparent.svg" "${work_dir}/data/transparent.png"

printf '%s\n' \
    'P3' \
    '2 2' \
    '255' \
    '255 0 0  0 255 0' \
    '0 0 255  255 255 255' > "${work_dir}/data/pattern.ppm"
vips copy "${work_dir}/data/pattern.ppm" "${work_dir}/data/pattern.png"
cp tests/waterfall_400.jpg "${work_dir}/data/with-exif.jpg"

printf '%s\n' \
    '%!PS-Adobe-3.0' \
    '%%BoundingBox: 0 0 40 20' \
    '%%Pages: 1' \
    '%%Page: 1 1' \
    'showpage' > "${work_dir}/blank.ps"
gs -q -dBATCH -dNOPAUSE -sDEVICE=pdfwrite \
    -sOutputFile="${work_dir}/data/blank.pdf" "${work_dir}/blank.ps"

printf '%s\n' \
    '%!PS-Adobe-3.0' \
    '%%BoundingBox: 0 0 40 20' \
    '%%Pages: 3' \
    '%%Page: 1 1' \
    'showpage' \
    '%%Page: 2 2' \
    'showpage' \
    '%%Page: 3 3' \
    'showpage' > "${work_dir}/pages.ps"
gs -q -dBATCH -dNOPAUSE -sDEVICE=pdfwrite \
    -sOutputFile="${work_dir}/data/pages.pdf" "${work_dir}/pages.ps"

# Files picturium cannot rasterise, for the raw-passthrough checks.
printf 'picturium\n' > "${work_dir}/data/notes.txt"
head -c 4096 /dev/urandom > "${work_dir}/data/archive.zip"
gzip -c "${work_dir}/data/source.svg" > "${work_dir}/data/source.svgz"
cp "${work_dir}/data/source.svgz" "${work_dir}/data/source.svg.gz"

export HOST=127.0.0.1
export PORT=20145
export PICTURIUM_CONFIG="${work_dir}/config.toml"
export PICTURIUM__SERVER__HOST="${HOST}"
export PICTURIUM__SERVER__PORT="${PORT}"
export PICTURIUM__DATA__DIR="${work_dir}/data"
export PICTURIUM__CACHE__DIR="${work_dir}/cache"
export PICTURIUM__CACHE__MEMORY__ENABLED=false
export PICTURIUM__CACHE__DISK__ENABLED=false
export PICTURIUM__SECURITY__SIGNATURE_ENABLED=false
export PICTURIUM__DATA__SERVE=txt
export PICTURIUM__IMAGE__UPSIZE=false
export RUSTFLAGS="-C linker-features=-lld"

cargo run > "${work_dir}/server.log" 2>&1 &
server_pid="$!"

for _ in $(seq 1 120); do
    if curl -fsS "http://${HOST}:${PORT}/health" >/dev/null; then
        break
    fi
    if ! kill -0 "${server_pid}" 2>/dev/null; then
        cat "${work_dir}/server.log"
        exit 1
    fi
    sleep 1
done
curl -fsS "http://${HOST}:${PORT}/health" >/dev/null

request() {
    local path="$1"
    local query="$2"
    local output="$3"
    echo "requesting ${path}?${query}"
    curl -fsS "http://${HOST}:${PORT}/${path}?${query}" -o "${output}"
}

assert_dimensions() {
    local image="$1"
    local expected_width="$2"
    local expected_height="$3"

    vips getpoint "${image}" "$((expected_width - 1))" "$((expected_height - 1))" >/dev/null
    if vips getpoint "${image}" "${expected_width}" 0 >/dev/null 2>&1; then
        return 1
    fi
    if vips getpoint "${image}" 0 "${expected_height}" >/dev/null 2>&1; then
        return 1
    fi
}

assert_pixel() {
    local image="$1"
    local x="$2"
    local y="$3"
    local red="$4"
    local green="$5"
    local blue="$6"
    local values
    values="$(vips getpoint "${image}" "${x}" "${y}")"
    awk -v values="${values}" -v r="${red}" -v g="${green}" -v b="${blue}" '
        BEGIN {
            split(values, pixel, /[[:space:]]+/)
            if ((pixel[1] - r)^2 > 16 || (pixel[2] - g)^2 > 16 || (pixel[3] - b)^2 > 16) {
                exit 1
            }
        }
    '
}

assert_transparent() {
    local image="$1"
    local x="$2"
    local y="$3"
    local values
    values="$(vips getpoint "${image}" "${x}" "${y}")"
    awk -v values="${values}" '
        BEGIN {
            count = split(values, pixel, /[[:space:]]+/)
            if (count < 4 || pixel[4] > 4) {
                exit 1
            }
        }
    '
}

assert_has_exif() {
    grep -aq 'Exif' "$1"
}

assert_no_exif() {
    if grep -aq 'Exif' "$1"; then
        return 1
    fi
}

assert_max_size() {
    local image="$1"
    local limit="$2"
    local size
    size="$(stat -c %s "${image}")"
    echo "  ${image} is ${size} B (limit ${limit} B)"
    [[ "${size}" -le "${limit}" ]]
}

assert_min_size() {
    local image="$1"
    local floor="$2"
    [[ "$(stat -c %s "$1")" -ge "${floor}" ]]
}

status_of() {
    curl -so /dev/null -w '%{http_code}' "http://${HOST}:${PORT}/$1?$2"
}

header_of() {
    curl -sD - -o /dev/null "http://${HOST}:${PORT}/$1?$2" \
        | tr -d '\r' \
        | awk -v name="$3:" 'tolower($1) == tolower(name) { $1 = ""; sub(/^ /, ""); print }'
}

pdf_pages() {
    gs -q -dNODISPLAY -dNOSAFER \
        -c "($1) (r) file runpdfbegin pdfpagecount = quit"
}

assert_status() {
    local actual
    actual="$(status_of "$1" "$2")"
    echo "  $1?$2 -> ${actual} (expected $3)"
    [[ "${actual}" == "$3" ]]
}

request source.png 'w=100&h=100&fit=contain&upsize=true&g=center&bg=ff0000&f=png' "${work_dir}/contain.png"
assert_dimensions "${work_dir}/contain.png" 100 100
assert_pixel "${work_dir}/contain.png" 0 0 255 0 0
assert_pixel "${work_dir}/contain.png" 50 50 0 0 255

request source.png 'w=100&h=100&fit=contain&upsize=true&g=top-left&bg=ff0000&f=png' "${work_dir}/gravity.png"
assert_pixel "${work_dir}/gravity.png" 0 0 0 0 255
assert_pixel "${work_dir}/gravity.png" 99 99 255 0 0

# Without upsizing the requested dimensions are a per-axis limit: a 40x20 source
# already fits in a 100x100 box, so it is served untouched and unpadded.
request source.png 'w=100&h=100&fit=contain&upsize=false&bg=ff0000&f=png' "${work_dir}/no-upsize.png"
assert_dimensions "${work_dir}/no-upsize.png" 40 20
assert_pixel "${work_dir}/no-upsize.png" 20 10 0 0 255
assert_pixel "${work_dir}/no-upsize.png" 0 0 0 0 255

# The height is capped by the source, the width limit still applies and pads.
request source.png 'w=100&h=10&fit=contain&upsize=false&bg=ff0000&f=png' "${work_dir}/no-upsize-limit.png"
assert_dimensions "${work_dir}/no-upsize-limit.png" 40 10
assert_pixel "${work_dir}/no-upsize-limit.png" 20 5 0 0 255
assert_pixel "${work_dir}/no-upsize-limit.png" 0 0 255 0 0

# jpeg is shrunk while decoding, which must not shrink the canvas with it.
request with-exif.jpg 'w=400&h=200&fit=contain&upsize=false&bg=ff0000&f=png' "${work_dir}/no-upsize-shrink.png"
assert_dimensions "${work_dir}/no-upsize-shrink.png" 400 200
assert_pixel "${work_dir}/no-upsize-shrink.png" 0 0 255 0 0

request source.png 'pad=2,3,4,5&bg=ff0000&f=png' "${work_dir}/padding.png"
assert_dimensions "${work_dir}/padding.png" 48 26
assert_pixel "${work_dir}/padding.png" 0 0 255 0 0
assert_pixel "${work_dir}/padding.png" 5 2 0 0 255

for format in png webp avif jxl gif; do
    output="${work_dir}/transparent-margin.${format}"
    request source.png "w=100&h=100&fit=contain&upsize=true&f=${format}" "${output}"
    assert_dimensions "${output}" 100 100
    assert_transparent "${output}" 0 0
done

request transparent.png 'pad=2&bg=00ff00&f=jpeg' "${work_dir}/flattened.jpg"
assert_pixel "${work_dir}/flattened.jpg" 5 5 0 255 0

request pattern.png 'pad=1&extend=bg&bg=000000&f=png' "${work_dir}/extend-bg.png"
assert_pixel "${work_dir}/extend-bg.png" 0 0 0 0 0
request pattern.png 'pad=1&extend=copy&f=png' "${work_dir}/extend-copy.png"
assert_pixel "${work_dir}/extend-copy.png" 0 0 255 0 0
request pattern.png 'pad=1&extend=repeat&f=png' "${work_dir}/extend-repeat.png"
assert_pixel "${work_dir}/extend-repeat.png" 0 0 255 255 255
request pattern.png 'pad=1&extend=mirror&f=png' "${work_dir}/extend-mirror.png"
assert_pixel "${work_dir}/extend-mirror.png" 0 0 255 0 0

request blank.pdf 'w=40&h=20&fit=contain&bg=ff0000&f=png' "${work_dir}/pdf-background.png"
assert_pixel "${work_dir}/pdf-background.png" 0 0 255 0 0

request with-exif.jpg 'f=jpeg&meta=none' "${work_dir}/meta-none.jpg"
assert_no_exif "${work_dir}/meta-none.jpg"
request with-exif.jpg 'f=jpeg&meta=exif' "${work_dir}/meta-exif.jpg"
assert_has_exif "${work_dir}/meta-exif.jpg"

# limit=size caps the encoded output by recompressing at a lower quality. The
# limit is derived from an unlimited encode so it stays reachable for every codec.
for format in jpeg webp avif jxl png; do
    base="${work_dir}/limit-base.${format}"
    request with-exif.jpg "q=maximum&f=${format}" "${base}"
    limit="$(( $(stat -c %s "${base}") * 3 / 5 ))"

    output="${work_dir}/limit-size.${format}"
    request with-exif.jpg "q=maximum&f=${format}&limit=size:${limit}" "${output}"
    assert_max_size "${output}" "${limit}"
    # A search that collapses to the quality floor still meets the limit, so also
    # check it did not throw away most of the budget getting there.
    assert_min_size "${output}" "$(( limit / 2 ))"
done

# `limits` is the canonical key, `limit` its documented alias.
request with-exif.jpg 'q=maximum&f=jpeg&limits=size:20000' "${work_dir}/limits-size.jpg"
assert_max_size "${work_dir}/limits-size.jpg" 20000

# Unit suffixes are binary: 20K == 20480 B.
request with-exif.jpg 'q=maximum&f=jpeg&limit=size:20K' "${work_dir}/limit-suffix.jpg"
assert_max_size "${work_dir}/limit-suffix.jpg" 20480
request with-exif.jpg 'q=maximum&f=jpeg&limit=size:1M' "${work_dir}/limit-suffix-large.jpg"
assert_max_size "${work_dir}/limit-suffix-large.jpg" 1048576

# A malformed size is a client error, not a 500.
status="$(curl -so /dev/null -w '%{http_code}' "http://${HOST}:${PORT}/with-exif.jpg?limit=size:20X")"
echo "  malformed limit returned ${status}"
[[ "${status}" == "400" ]]

# An unreachable limit must still serve an image rather than fail the request.
request with-exif.jpg 'f=jpeg&limit=size:200' "${work_dir}/limit-unreachable.jpg"
assert_dimensions "${work_dir}/limit-unreachable.jpg" 400 400

# GIF has no quality knob, so the limit is ignored instead of looping forever.
request with-exif.jpg 'f=gif&limit=size:200' "${work_dir}/limit-gif.gif"
assert_dimensions "${work_dir}/limit-gif.gif" 400 400

# --- encoder configuration ------------------------------------------------
# Config only reaches the encoders through OutputConfig, so prove it end to end
# with a second server whose jpeg curve is squashed, and prove a bad value is
# refused at startup rather than per request.

request with-exif.jpg 'q=maximum&f=jpeg' "${work_dir}/tuned-baseline.jpg"

tuned_dir="$(mktemp -d)"
PICTURIUM__OUTPUT__QUALITY_CURVES__JPEG__MAX=30 \
    PICTURIUM__OUTPUT__QUALITY_CURVES__JPEG__MAXIMUM=0 \
    PICTURIUM__SERVER__PORT=20146 \
    cargo run > "${tuned_dir}/server.log" 2>&1 &
tuned_pid="$!"
for _ in $(seq 1 120); do
    curl -fsS "http://${HOST}:20146/health" >/dev/null 2>&1 && break
    sleep 1
done
curl -fsS "http://${HOST}:20146/with-exif.jpg?q=maximum&f=jpeg" -o "${work_dir}/tuned.jpg"
kill "${tuned_pid}" 2>/dev/null || true
wait "${tuned_pid}" 2>/dev/null || true
rm -rf "${tuned_dir}"

baseline_size="$(stat -c %s "${work_dir}/tuned-baseline.jpg")"
tuned_size="$(stat -c %s "${work_dir}/tuned.jpg")"
echo "  jpeg curve override: ${baseline_size} B -> ${tuned_size} B"
[[ "${tuned_size}" -lt "${baseline_size}" ]]

# An out-of-range knob must abort startup, naming the variable.
if PICTURIUM__OUTPUT__ENCODER__AVIF__BITDEPTH=7 PICTURIUM__SERVER__PORT=20147 \
    cargo run > "${work_dir}/invalid.log" 2>&1; then
    echo "server started with an invalid avif bitdepth"
    exit 1
fi
grep -q 'output.encoder.avif.bitdepth' "${work_dir}/invalid.log"
echo "  invalid output.encoder.avif.bitdepth refused at startup"

# --- document output (f=pdf / f=svg) ---------------------------------------
# libvips cannot write PDF or SVG, so these serve the document itself.

request pages.pdf 'f=pdf' "${work_dir}/whole.pdf"
cmp "${work_dir}/whole.pdf" "${work_dir}/data/pages.pdf"
[[ "$(header_of pages.pdf 'f=pdf' content-type)" == "application/pdf" ]]
echo "  f=pdf served the source document unchanged"

# `thumb` selects exact pages, not a contiguous range.
request pages.pdf 'f=pdf&thumb=p:1,3' "${work_dir}/subset.pdf"
subset_pages="$(pdf_pages "${work_dir}/subset.pdf")"
echo "  f=pdf&thumb=p:1,3 produced ${subset_pages} pages"
[[ "${subset_pages}" == "2" ]]

# Asking for every page rewrites nothing.
request pages.pdf 'f=pdf&thumb=p:1,2,3' "${work_dir}/all-pages.pdf"
cmp "${work_dir}/all-pages.pdf" "${work_dir}/data/pages.pdf"

# Pages past the end are dropped; a selection with none of them is a client error.
request pages.pdf 'f=pdf&thumb=p:3,9' "${work_dir}/past-end.pdf"
[[ "$(pdf_pages "${work_dir}/past-end.pdf")" == "1" ]]
assert_status pages.pdf 'f=pdf&thumb=p:9' 400

request source.svg 'f=svg' "${work_dir}/passthrough.svg"
cmp "${work_dir}/passthrough.svg" "${work_dir}/data/source.svg"
[[ "$(header_of source.svg 'f=svg' content-type)" == "image/svg+xml" ]]

# `.svgz` and `.svg.gz` are both gzipped SVG and must say so.
[[ "$(header_of source.svgz 'f=svg' content-encoding)" == "gzip" ]]
[[ "$(header_of source.svg.gz 'f=svg' content-encoding)" == "gzip" ]]
[[ "$(header_of source.svg.gz 'f=svg' content-type)" == "image/svg+xml" ]]
echo "  svgz and svg.gz passthrough are marked gzip"

# `.svg.gz` has a `gz` extension but must still rasterise as a vector source.
request source.svg.gz 'w=40&h=20&f=png' "${work_dir}/from-svg-gz.png"
assert_dimensions "${work_dir}/from-svg-gz.png" 40 20
assert_pixel "${work_dir}/from-svg-gz.png" 20 10 0 0 255

# A raster source cannot become a document.
assert_status source.png 'f=pdf' 415
assert_status source.png 'f=svg' 415

# Rasterising a vector source still works.
request source.svg 'w=40&h=20&f=png' "${work_dir}/rasterised.png"
assert_dimensions "${work_dir}/rasterised.png" 40 20
assert_pixel "${work_dir}/rasterised.png" 20 10 0 0 255

# --- raw passthrough (data.serve) ------------------------------------------
# `txt` is allowlisted above, `zip` is not.

request notes.txt '' "${work_dir}/notes.txt"
cmp "${work_dir}/notes.txt" "${work_dir}/data/notes.txt"
[[ "$(header_of notes.txt '' content-type)" == "text/plain; charset=utf-8" ]]
assert_status archive.zip '' 415

# The passthrough path keeps its conditional and range handling.
range_status="$(curl -so "${work_dir}/range.bin" -w '%{http_code}' \
    -H 'Range: bytes=0-3' "http://${HOST}:${PORT}/notes.txt")"
echo "  ranged passthrough returned ${range_status}"
[[ "${range_status}" == "206" ]]
[[ "$(stat -c %s "${work_dir}/range.bin")" == "4" ]]

etag="$(header_of notes.txt '' etag)"
[[ -n "${etag}" ]]
conditional="$(curl -so /dev/null -w '%{http_code}' \
    -H "If-None-Match: ${etag}" "http://${HOST}:${PORT}/notes.txt")"
echo "  conditional passthrough returned ${conditional}"
[[ "${conditional}" == "304" ]]

echo "bg pipeline checks passed"
