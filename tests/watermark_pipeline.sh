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
        bash tests/watermark_pipeline.sh --inside
fi

work_dir="$(mktemp -d /tmp/picturium-watermark.XXXXXX)"
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

# A white 200x200 canvas, so anything the watermark draws stands out.
printf '%s\n' \
    '<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200">' \
    '<rect width="200" height="200" fill="#ffffff"/>' \
    '</svg>' > "${work_dir}/source.svg"
vips copy "${work_dir}/source.svg" "${work_dir}/data/source.png"

# An opaque blue 20x20 mark.
printf '%s\n' \
    '<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20">' \
    '<rect width="20" height="20" fill="#0000ff"/>' \
    '</svg>' > "${work_dir}/logo.svg"
vips copy "${work_dir}/logo.svg" "${work_dir}/data/logo.png"

# A two frame animation and a two page document, so multi-page output has a
# second page that must be marked as well. The frames have to differ, or the
# GIF encoder folds them back into a single frame.
printf '%s\n' \
    '<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200">' \
    '<rect width="200" height="200" fill="#ffffff"/>' \
    '<rect x="90" y="90" width="20" height="20" fill="#000000"/>' \
    '</svg>' > "${work_dir}/frame2.svg"
vips copy "${work_dir}/frame2.svg" "${work_dir}/frame2.png"
vips arrayjoin "${work_dir}/data/source.png ${work_dir}/frame2.png" "${work_dir}/strip.v" --across 1
vips gifsave "${work_dir}/strip.v" "${work_dir}/data/animated.gif" --page-height 200

printf '%s\n' \
    '%!PS-Adobe-3.0' \
    '%%BoundingBox: 0 0 200 200' \
    '%%Pages: 2' \
    '%%Page: 1 1' \
    'showpage' \
    '%%Page: 2 2' \
    'showpage' > "${work_dir}/pages.ps"
gs -q -dBATCH -dNOPAUSE -sDEVICE=pdfwrite \
    -dDEVICEWIDTHPOINTS=200 -dDEVICEHEIGHTPOINTS=200 -dFIXEDMEDIA \
    -sOutputFile="${work_dir}/data/pages.pdf" "${work_dir}/pages.ps"

# A mark twice the size of the canvas, to check it is shrunk instead of refused.
printf '%s\n' \
    '<svg xmlns="http://www.w3.org/2000/svg" width="400" height="400">' \
    '<rect width="400" height="400" fill="#0000ff"/>' \
    '</svg>' > "${work_dir}/big.svg"
vips copy "${work_dir}/big.svg" "${work_dir}/data/big.png"

export HOST=127.0.0.1
export PORT=20146
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

request() {
    request_file source.png "$1" "$2"
}

status_of() {
    curl -s -o /dev/null -w '%{http_code}' "http://${HOST}:${PORT}/source.png?$1"
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

# How much of the white canvas the blue mark covers: the mark has no red in it,
# so the lower the mean of the red channel, the more of it is drawn.
coverage() {
    vips extract_band "$1" "${work_dir}/red.v" 0 >/dev/null
    awk -v mean="$(vips avg "${work_dir}/red.v")" 'BEGIN { print 255 - mean }'
}

# The mark is drawn where it was anchored, and nowhere else.
start_server
request 'f=png&watermark=image:logo.png|anchor:bottom-right|pad:10|opacity:100' "${work_dir}/corner.png"
assert_pixel "${work_dir}/corner.png" 180 180 0 0 255
assert_pixel "${work_dir}/corner.png" 5 5 255 255 255

request 'f=png&watermark=image:logo.png|anchor:top-left|pad:10|opacity:100' "${work_dir}/topleft.png"
assert_pixel "${work_dir}/topleft.png" 15 15 0 0 255
assert_pixel "${work_dir}/topleft.png" 195 195 255 255 255

# Opacity fades the mark towards the white canvas.
request 'f=png&watermark=image:logo.png|anchor:center|opacity:100' "${work_dir}/opaque.png"
request 'f=png&watermark=image:logo.png|anchor:center|opacity:20' "${work_dir}/faded.png"
assert_pixel "${work_dir}/opaque.png" 100 100 0 0 255
opaque_red="$(vips getpoint "${work_dir}/opaque.png" 100 100 | awk '{print $1}')"
faded_red="$(vips getpoint "${work_dir}/faded.png" 100 100 | awk '{print $1}')"
echo "  opacity 100 red=${opaque_red}, opacity 20 red=${faded_red}"
awk -v a="${opaque_red}" -v b="${faded_red}" 'BEGIN { exit !(b > a + 50) }'

# Repeating covers far more of the canvas than a single anchored mark.
request 'f=png&watermark=image:logo.png|anchor:repeat|pad:5|opacity:100' "${work_dir}/repeat.png"
assert_pixel "${work_dir}/repeat.png" 15 15 0 0 255
assert_pixel "${work_dir}/repeat.png" 45 45 0 0 255
single="$(coverage "${work_dir}/corner.png")"
repeated="$(coverage "${work_dir}/repeat.png")"
echo "  coverage: single=${single}, repeat=${repeated}"
awk -v a="${single}" -v b="${repeated}" 'BEGIN { exit !(b > a * 4) }'

# A 45 degree rotation turns the square into a diamond: the centre stays covered
# while the corners of the original square are freed.
request 'f=png&watermark=image:logo.png|anchor:center|rot:45|opacity:100' "${work_dir}/rotated.png"
assert_pixel "${work_dir}/rotated.png" 100 100 0 0 255
assert_pixel "${work_dir}/rotated.png" 91 91 255 255 255
assert_pixel "${work_dir}/opaque.png" 91 91 0 0 255

# Text is rendered in the requested colour.
request 'f=png&watermark=text:HELLO|color:ff0000|size:48|anchor:center' "${work_dir}/text.png"
red_mean="$(vips extract_band "${work_dir}/text.png" "${work_dir}/textred.v" 0 >/dev/null; vips avg "${work_dir}/textred.v")"
green_mean="$(vips extract_band "${work_dir}/text.png" "${work_dir}/green.v" 1 >/dev/null; vips avg "${work_dir}/green.v")"
echo "  text means: red=${red_mean}, green=${green_mean}"
awk -v r="${red_mean}" -v g="${green_mean}" 'BEGIN { exit !(r > g + 1) }'

# A request supplied path may not escape the data directory.
traversal_status="$(status_of 'f=png&watermark=image:../../etc/passwd')"
echo "  traversal status: ${traversal_status}"
[[ "${traversal_status}" == "500" ]]
grep -aq 'watermark image not found' "${work_dir}/server.log"

# A mark larger than the image is shrunk to fit, padding included.
request 'f=png&watermark=image:big.png|anchor:center|pad:0|opacity:100' "${work_dir}/oversized.png"
assert_pixel "${work_dir}/oversized.png" 0 0 0 0 255
assert_pixel "${work_dir}/oversized.png" 199 199 0 0 255

request 'f=png&watermark=image:big.png|anchor:center|pad:10|opacity:100' "${work_dir}/oversized-pad.png"
assert_pixel "${work_dir}/oversized-pad.png" 100 100 0 0 255
assert_pixel "${work_dir}/oversized-pad.png" 15 15 0 0 255
assert_pixel "${work_dir}/oversized-pad.png" 4 4 255 255 255
assert_pixel "${work_dir}/oversized-pad.png" 195 195 255 255 255

# Padding with no room left for the mark is refused rather than guessed at.
padding_status="$(status_of 'f=png&watermark=image:logo.png|pad:120')"
echo "  padding status: ${padding_status}"
[[ "${padding_status}" == "500" ]]
grep -aq 'watermark padding leaves no room' "${work_dir}/server.log"

# Every frame of an animation is marked, not just the first one.
mark='watermark=image:logo.png|anchor:bottom-right|pad:10|opacity:100'
request_file animated.gif "f=gif&animate=frames:-1&${mark}" "${work_dir}/animated.gif"
vips copy "${work_dir}/animated.gif[n=-1]" "${work_dir}/animated.v"
assert_pixel "${work_dir}/animated.v" 180 180 0 0 255
assert_pixel "${work_dir}/animated.v" 180 380 0 0 255
assert_pixel "${work_dir}/animated.v" 5 5 255 255 255
assert_pixel "${work_dir}/animated.v" 5 205 255 255 255

# So is every rendered page of a document.
request_file pages.pdf "f=png&pages=1-2&w=200&bg=ffffff&${mark}" "${work_dir}/pages.png"
assert_pixel "${work_dir}/pages.png" 180 180 0 0 255
assert_pixel "${work_dir}/pages.png" 180 380 0 0 255

# `watermark=false` and an unrelated parameter both leave the image untouched.
request 'f=png' "${work_dir}/plain.png"
assert_pixel "${work_dir}/plain.png" 180 180 255 255 255
stop_server

# The configuration switches the watermark on for every request.
export PICTURIUM__WATERMARK__ENABLED=true
export PICTURIUM__WATERMARK__IMAGE__PATH="${work_dir}/data/logo.png"
export PICTURIUM__WATERMARK__POSITION=bottom-right
export PICTURIUM__WATERMARK__OPACITY=100
export PICTURIUM__WATERMARK__MAX_SCALE=0.5
start_server
request 'f=png' "${work_dir}/config-on.png"
assert_pixel "${work_dir}/config-on.png" 185 185 0 0 255
request 'f=png&watermark=false' "${work_dir}/config-off.png"
assert_pixel "${work_dir}/config-off.png" 185 185 255 255 255

# max_scale keeps an oversized mark to half the image, centred in the padding
# free box it is given.
request 'f=png&watermark=image:big.png|anchor:center|pad:0' "${work_dir}/capped.png"
assert_pixel "${work_dir}/capped.png" 100 100 0 0 255
assert_pixel "${work_dir}/capped.png" 60 60 0 0 255
assert_pixel "${work_dir}/capped.png" 40 40 255 255 255
assert_pixel "${work_dir}/capped.png" 160 160 255 255 255
stop_server

# Enabling the watermark without anything to draw refuses to boot.
export PICTURIUM__WATERMARK__IMAGE__PATH=""
if cargo run > "${work_dir}/boot.log" 2>&1; then
    cat "${work_dir}/boot.log"
    echo "expected the server to refuse to start"
    exit 1
fi
grep -q 'watermark.enabled is on' "${work_dir}/boot.log"

echo "watermark pipeline checks passed"
