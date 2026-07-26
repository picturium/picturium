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

export HOST=127.0.0.1
export PORT=20145
export DATA_DIR="${work_dir}/data"
export CACHE_DIR="${work_dir}/cache"
export CACHE_MEMORY_ENABLED=false
export CACHE_DISK_ENABLED=false
export SIGNATURE_ENABLED=false
export IMAGE_UPSIZE=false
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

request source.png 'w=100&h=100&fit=contain&upsize=true&g=center&bg=ff0000&f=png' "${work_dir}/contain.png"
assert_dimensions "${work_dir}/contain.png" 100 100
assert_pixel "${work_dir}/contain.png" 0 0 255 0 0
assert_pixel "${work_dir}/contain.png" 50 50 0 0 255

request source.png 'w=100&h=100&fit=contain&upsize=true&g=top-left&bg=ff0000&f=png' "${work_dir}/gravity.png"
assert_pixel "${work_dir}/gravity.png" 0 0 0 0 255
assert_pixel "${work_dir}/gravity.png" 99 99 255 0 0

request source.png 'w=100&h=100&fit=contain&upsize=false&bg=ff0000&f=png' "${work_dir}/no-upsize.png"
assert_dimensions "${work_dir}/no-upsize.png" 100 100
assert_pixel "${work_dir}/no-upsize.png" 50 50 0 0 255
assert_pixel "${work_dir}/no-upsize.png" 0 0 255 0 0

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

echo "bg pipeline checks passed"
