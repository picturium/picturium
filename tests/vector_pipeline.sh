#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" != "--inside" ]]; then
    repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    exec docker run --rm \
        -v "${repo_root}:/root/picturium" \
        -v "${repo_root}/../picturium-libvips:/root/picturium-libvips" \
        -v /usr/share/fonts:/usr/share/fonts \
        -w /root/picturium \
        lamka02sk/picturium-dev:8.18.4 \
        bash tests/vector_pipeline.sh --inside
fi

work_dir="$(mktemp -d /tmp/picturium-vector.XXXXXX)"
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

# A blue rectangle, wider than it is tall so a transposed render is obvious.
printf '%s\n' \
    '<svg xmlns="http://www.w3.org/2000/svg" width="144" height="72">' \
    '<rect width="144" height="72" fill="#0000ff"/>' \
    '</svg>' > "${work_dir}/source.svg"

# Inkscape writes the fixture itself: .eps is the only one of the five vector
# formats anything in this image can produce. .ai and .cdr are covered by the
# format detection unit tests and need real files to check by hand.
inkscape --export-type=eps --export-filename="${work_dir}/data/source.eps" \
    "${work_dir}/source.svg" >/dev/null 2>&1
[[ -s "${work_dir}/data/source.eps" ]] || { echo "  fixture .eps was not produced"; exit 1; }

# A modern .ai file is a PDF wrapper, which is the path Inkscape takes for it.
inkscape --export-type=pdf --export-filename="${work_dir}/artboard.pdf" \
    "${work_dir}/source.svg" >/dev/null 2>&1
cp "${work_dir}/artboard.pdf" "${work_dir}/data/artboard.ai"
[[ -s "${work_dir}/data/artboard.ai" ]] || { echo "  fixture .ai was not produced"; exit 1; }

export HOST=127.0.0.1
export PORT=20147
export PICTURIUM_CONFIG="${work_dir}/config.toml"
export PICTURIUM__SERVER__HOST="${HOST}"
export PICTURIUM__SERVER__PORT="${PORT}"
export PICTURIUM__DATA__DIR="${work_dir}/data"
export PICTURIUM__CACHE__DIR="${work_dir}/cache"
export PICTURIUM__SECURITY__SIGNATURE_ENABLED=false
export PICTURIUM__SERVER__LOG_LEVEL=warn
export RUSTFLAGS="-C linker-features=-lld"

start_server() {
    cargo run > "${work_dir}/server.log" 2>&1 &
    server_pid="$!"

    for _ in $(seq 1 120); do
        if curl -fsS "http://${HOST}:${PORT}/health" >/dev/null 2>&1; then
            return 0
        fi
        if ! kill -0 "${server_pid}" 2>/dev/null; then
            cat "${work_dir}/server.log"
            return 1
        fi
        sleep 1
    done

    cat "${work_dir}/server.log"
    return 1
}

stop_server() {
    if [[ -n "${server_pid}" ]]; then
        kill "${server_pid}" 2>/dev/null || true
        wait "${server_pid}" 2>/dev/null || true
        server_pid=""
    fi
}

request_file() {
    local file="$1" query="$2" output="$3"
    echo "requesting ${file}?${query}"
    curl -fsS "http://${HOST}:${PORT}/${file}?${query}" -o "${output}"
}

status_of() {
    local file="$1" query="$2"
    curl -s -o /dev/null -w '%{http_code}' "http://${HOST}:${PORT}/${file}?${query}"
}

assert_pixel() {
    local image="$1" x="$2" y="$3" red="$4" green="$5" blue="$6"
    local values
    values="$(vips getpoint "${image}" "${x}" "${y}")"
    awk -v values="${values}" -v r="${red}" -v g="${green}" -v b="${blue}" -v where="${image} @ ${x},${y}" '
        BEGIN {
            split(values, pixel, /[[:space:]]+/)
            if ((pixel[1] - r)^2 > 64 || (pixel[2] - g)^2 > 64 || (pixel[3] - b)^2 > 64) {
                printf "  %s is %s, expected %s %s %s\n", where, values, r, g, b
                exit 1
            }
        }
    '
}

width_of() {
    vips im_header_int width "$1"
}

start_server

echo "== an EPS source rasterizes through the vector pipeline"
request_file source.eps "f=png" "${work_dir}/eps.png"
[[ "$(head -c4 "${work_dir}/eps.png" | tail -c3)" == "PNG" ]] || {
    echo "  f=png did not return a PNG"; exit 1
}
assert_pixel "${work_dir}/eps.png" 10 10 0 0 255

echo "== an AI source rasterizes through the vector pipeline"
request_file artboard.ai "f=png" "${work_dir}/ai.png"
assert_pixel "${work_dir}/ai.png" 10 10 0 0 255

echo "== the intermediate stays vector, so dpi rescales instead of upsizing"
request_file source.eps "f=png&dpi=72" "${work_dir}/dpi72.png"
request_file source.eps "f=png&dpi=144" "${work_dir}/dpi144.png"
awk -v low="$(width_of "${work_dir}/dpi72.png")" -v high="$(width_of "${work_dir}/dpi144.png")" '
    BEGIN {
        if (high != low * 2) {
            printf "  dpi=144 is %d wide, expected twice the %d of dpi=72\n", high, low
            exit 1
        }
    }
'

echo "== f=pdf serves the converted document"
request_file source.eps "f=pdf" "${work_dir}/out.pdf"
[[ "$(head -c4 "${work_dir}/out.pdf")" == "%PDF" ]] || { echo "  f=pdf did not return a PDF"; exit 1; }

echo "== f=svg serves plain SVG converted by inkscape"
request_file source.eps "f=svg" "${work_dir}/out.svg"
grep -aq "<svg" "${work_dir}/out.svg" || { echo "  f=svg did not return an SVG"; exit 1; }
grep -aq "inkscape:" "${work_dir}/out.svg" && { echo "  f=svg kept inkscape-specific attributes"; exit 1; }

echo "== a raster source is still refused for f=svg"
vips copy "${work_dir}/source.svg" "${work_dir}/data/raster.png"
[[ "$(status_of raster.png "f=svg")" == "415" ]] || { echo "  f=svg on a PNG should be 415"; exit 1; }

# Whether the second render reused the cached PDF or reconverted is only visible
# in timing, so this checks the weaker thing that is actually observable: the
# conversion is cached at all, and a repeat render off it is still correct.
echo "== the conversion is cached and repeat renders come out the same"
[[ "$(find "${work_dir}/cache" -type f | wc -l)" -gt 0 ]] || {
    echo "  nothing was written to the cache"; exit 1
}
request_file source.eps "f=png" "${work_dir}/again.png"
cmp -s "${work_dir}/eps.png" "${work_dir}/again.png" || {
    echo "  a cached render differs from the first one"; exit 1
}

echo "== a conversion that outruns its timeout fails instead of hanging"
stop_server
export PICTURIUM__VECTOR__CONVERSION_TIMEOUT=0
rm -rf "${work_dir}/cache"
mkdir -p "${work_dir}/cache"
start_server
[[ "$(status_of source.eps "f=png")" == "500" ]] || { echo "  a timed out conversion should be 500"; exit 1; }
sleep 1
pgrep -x inkscape >/dev/null && { echo "  inkscape was left running after the timeout"; exit 1; }
unset PICTURIUM__VECTOR__CONVERSION_TIMEOUT

echo "vector checks passed"
