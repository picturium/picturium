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
        bash tests/animation_pipeline.sh --inside
fi

work_dir="$(mktemp -d /tmp/picturium-animation.XXXXXX)"
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

# Four 100x100 frames in distinct solid colours, so a frame can be identified
# from a single pixel and the encoder cannot fold them into one another.
frame() {
    local name="$1" fill="$2"
    printf '%s\n' \
        '<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">' \
        "<rect width=\"100\" height=\"100\" fill=\"${fill}\"/>" \
        '</svg>' > "${work_dir}/${name}.svg"
    vips copy "${work_dir}/${name}.svg" "${work_dir}/${name}.png"
}

frame f0 '#ffffff'
frame f1 '#ff0000'
frame f2 '#00ff00'
frame f3 '#0000ff'

vips arrayjoin \
    "${work_dir}/f0.png ${work_dir}/f1.png ${work_dir}/f2.png ${work_dir}/f3.png" \
    "${work_dir}/strip.v" --across 1
vips gifsave "${work_dir}/strip.v" "${work_dir}/data/animated.gif" --page-height 100
vips webpsave "${work_dir}/strip.v" "${work_dir}/data/animated.webp" --page-height 100 --lossless
cp "${work_dir}/f0.png" "${work_dir}/data/still.png"

# Two frames that differ only in their background, each with the same
# high-contrast blob near the right edge: a centre crop cuts the blob off, a
# smart crop would keep it, and the two backgrounds tell the frames apart.
blob_frame() {
    local name="$1" fill="$2"
    printf '%s\n' \
        '<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100">' \
        "<rect width=\"200\" height=\"100\" fill=\"${fill}\"/>" \
        '<circle cx="185" cy="50" r="12" fill="#ff0000"/>' \
        '</svg>' > "${work_dir}/${name}.svg"
    vips copy "${work_dir}/${name}.svg" "${work_dir}/${name}.png"
}

blob_frame b0 '#808080'
blob_frame b1 '#909090'
vips arrayjoin "${work_dir}/b0.png ${work_dir}/b1.png" "${work_dir}/blob-strip.v" --across 1
vips gifsave "${work_dir}/blob-strip.v" "${work_dir}/data/blob-animated.gif" --page-height 100

# Three seconds of moving test pattern, to extract a clip from.
ffmpeg -nostdin -v error -f lavfi -i testsrc=duration=3:size=100x100:rate=25 \
    -pix_fmt yuv420p -y "${work_dir}/data/clip.mp4"

# Forty seconds in four solid-colour ten-second blocks, so the timestamp a
# sampled frame came from can be read straight off its colour. Sampling this
# far apart is what sends a clip down the seek-per-sample path.
ffmpeg -nostdin -v error \
    -f lavfi -i "color=c=red:s=100x100:d=10:r=10" \
    -f lavfi -i "color=c=lime:s=100x100:d=10:r=10" \
    -f lavfi -i "color=c=blue:s=100x100:d=10:r=10" \
    -f lavfi -i "color=c=white:s=100x100:d=10:r=10" \
    -filter_complex "[0:v][1:v][2:v][3:v]concat=n=4:v=1:a=0" \
    -pix_fmt yuv420p -g 25 -y "${work_dir}/data/blocks.mp4"

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

    for _ in $(seq 1 180); do
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

request_file() {
    local file="$1" query="$2" output="$3"
    curl -fsS "http://${HOST}:${PORT}/${file}?${query}" -o "${output}"
}

# The dev image ships no `vipsheader`, so metadata is read back out of a vips
# file, which stores it as XML in the trailer.
meta() {
    local source="$1" field="$2"
    rm -f "${work_dir}/meta.v"
    vips copy "${source}" "${work_dir}/meta.v" >/dev/null 2>&1
    strings -a "${work_dir}/meta.v" \
        | grep -o "name=\"${field}\">[^<]*" \
        | head -1 \
        | sed "s/name=\"${field}\">//" \
        | sed 's/[[:space:]]*$//'
}

failures=0

check() {
    local label="$1" actual="$2" expected="$3"
    if [[ "${actual}" == "${expected}" ]]; then
        echo "  ok   ${label}"
    else
        echo "  FAIL ${label}: got '${actual}', expected '${expected}'"
        failures=$((failures + 1))
    fi
}

assert_pixel() {
    local label="$1" image="$2" x="$3" y="$4" red="$5" green="$6" blue="$7"
    local values
    values="$(vips getpoint "${image}" "${x}" "${y}")"
    if awk -v values="${values}" -v r="${red}" -v g="${green}" -v b="${blue}" '
        BEGIN {
            split(values, pixel, /[[:space:]]+/)
            exit ((pixel[1] - r)^2 > 64 || (pixel[2] - g)^2 > 64 || (pixel[3] - b)^2 > 64)
        }'
    then
        echo "  ok   ${label}"
    else
        echo "  FAIL ${label}: ${image} @ ${x},${y} is ${values}, expected ${red} ${green} ${blue}"
        failures=$((failures + 1))
    fi
}

start_server

out="${work_dir}/out"

echo "an animated source stays animated without any anim parameter"
request_file animated.gif "" "${out}.webp"
check "frame count" "$(meta "${out}.webp[n=-1]" n-pages)" "4"
check "frame height" "$(meta "${out}.webp[n=-1]" page-height)" "100"

echo "the frames keep their order and content"
vips copy "${out}.webp[n=-1]" "${out}-frames.v"
assert_pixel "frame 1 is white" "${out}-frames.v" 50 50 255 255 255
assert_pixel "frame 2 is red" "${out}-frames.v" 50 150 255 0 0
assert_pixel "frame 3 is green" "${out}-frames.v" 50 250 0 255 0
assert_pixel "frame 4 is blue" "${out}-frames.v" 50 350 0 0 255

echo "gif output carries the frames too"
request_file animated.gif "f=gif" "${out}.gif"
check "frame count" "$(meta "${out}.gif[n=-1]" n-pages)" "4"

echo "an animated webp source survives"
request_file animated.webp "f=gif" "${out}-webp-in.gif"
check "frame count" "$(meta "${out}-webp-in.gif[n=-1]" n-pages)" "4"

echo "a smart gravity falls back to the centre on an animation"
# Smart crop is picked per image, so on an animation it would land somewhere
# else on every frame. Both frames must keep their centre, blob cut off.
request_file blob-animated.gif "w=100&h=100&fit=cover&g=attention&f=gif" "${out}-smart.gif"
check "frame count" "$(meta "${out}-smart.gif[n=-1]" n-pages)" "2"
vips copy "${out}-smart.gif[n=-1]" "${out}-smart.v"
assert_pixel "frame 1 kept its centre" "${out}-smart.v" 95 50 128 128 128
assert_pixel "frame 2 kept its centre" "${out}-smart.v" 95 150 144 144 144

request_file blob-animated.gif "w=100&h=100&fit=cover&g=entropy&f=gif" "${out}-smart-entropy.gif"
vips copy "${out}-smart-entropy.gif[n=-1]" "${out}-smart-entropy.v"
assert_pixel "entropy falls back too" "${out}-smart-entropy.v" 95 50 128 128 128

# The compass gravities do apply per frame, so g=right keeps the blob on both.
request_file blob-animated.gif "w=100&h=100&fit=cover&g=right&f=gif" "${out}-right.gif"
vips copy "${out}-right.gif[n=-1]" "${out}-right.v"
assert_pixel "frame 1 kept the blob" "${out}-right.v" 95 50 255 0 0
assert_pixel "frame 2 kept the blob" "${out}-right.v" 95 150 255 0 0

echo "a format that cannot animate gets the first frame"
for format in jpeg png avif jxl; do
    request_file animated.gif "f=${format}" "${out}-flat"
    check "${format} height" "$(vips getpoint "${out}-flat" 0 99 >/dev/null 2>&1 && echo ok)" "ok"
    if vips getpoint "${out}-flat" 0 100 >/dev/null 2>&1; then
        echo "  FAIL ${format} kept more than one frame"
        failures=$((failures + 1))
    else
        echo "  ok   ${format} is a single frame"
    fi
done

echo "resizing rescales every frame and re-tags the frame height"
request_file animated.gif "f=gif&w=50&h=50" "${out}-small.gif"
check "frame count" "$(meta "${out}-small.gif[n=-1]" n-pages)" "4"
check "frame height" "$(meta "${out}-small.gif[n=-1]" page-height)" "50"

echo "a cover crop applies to each frame"
request_file animated.gif "f=gif&w=60&h=40&fit=cover" "${out}-cover.gif"
check "frame count" "$(meta "${out}-cover.gif[n=-1]" n-pages)" "4"
check "frame height" "$(meta "${out}-cover.gif[n=-1]" page-height)" "40"

echo "padding surrounds each frame, not the strip"
request_file animated.gif "f=gif&pad=10&bg=000000" "${out}-pad.gif"
check "frame count" "$(meta "${out}-pad.gif[n=-1]" n-pages)" "4"
check "frame height" "$(meta "${out}-pad.gif[n=-1]" page-height)" "120"
vips copy "${out}-pad.gif[n=-1]" "${out}-pad.v"
assert_pixel "frame 2 keeps its own padding" "${out}-pad.v" 5 125 0 0 0
assert_pixel "frame 2 content is still red" "${out}-pad.v" 60 180 255 0 0

echo "a crop applies to each frame"
request_file animated.gif "f=gif&crop=w:40|h:40|g:center" "${out}-crop.gif"
check "frame count" "$(meta "${out}-crop.gif[n=-1]" n-pages)" "4"
check "frame height" "$(meta "${out}-crop.gif[n=-1]" page-height)" "40"

echo "rotation turns every frame"
request_file animated.gif "f=gif&rot=90&w=80&h=40" "${out}-rot.gif"
check "frame count" "$(meta "${out}-rot.gif[n=-1]" n-pages)" "4"

echo "a per-pixel filter leaves the frames alone"
request_file animated.gif "f=gif&filter=grayscale:1" "${out}-gray.gif"
check "frame count" "$(meta "${out}-gray.gif[n=-1]" n-pages)" "4"
check "frame height" "$(meta "${out}-gray.gif[n=-1]" page-height)" "100"

echo "a filter that reads neighbouring pixels runs frame by frame"
request_file animated.gif "f=gif&filter=blur:5" "${out}-blur.gif"
check "frame count" "$(meta "${out}-blur.gif[n=-1]" n-pages)" "4"
vips copy "${out}-blur.gif[n=-1]" "${out}-blur.v"
# The seam between two frames would smear if the blur ran across the strip.
assert_pixel "frame 2 is still red at its top edge" "${out}-blur.v" 50 101 255 0 0
assert_pixel "frame 1 is still white at its bottom edge" "${out}-blur.v" 50 98 255 255 255

echo "a pixelate filter re-tags the frame height"
request_file animated.gif "f=gif&filter=pixelate:5" "${out}-pix.gif"
check "frame count" "$(meta "${out}-pix.gif[n=-1]" n-pages)" "4"
check "frame height" "$(meta "${out}-pix.gif[n=-1]" page-height)" "100"

echo "a start frame past the end clamps to the last frame"
request_file animated.gif "f=gif&pages=99" "${out}-past.gif"
check "frame count" "$(meta "${out}-past.gif[n=-1]" n-pages)" "1"
vips copy "${out}-past.gif[n=-1]" "${out}-past.v"
assert_pixel "clamped to the last frame" "${out}-past.v" 50 50 0 0 255

echo "timing overrides the source delays"
request_file animated.gif "f=gif&anim=timing:250" "${out}-timing.gif"
check "delays" "$(meta "${out}-timing.gif[n=-1]" delay)" "250 250 250 250"

echo "fps is the same control expressed as a rate"
request_file animated.gif "f=gif&anim=fps:10" "${out}-fps.gif"
check "delays" "$(meta "${out}-fps.gif[n=-1]" delay)" "100 100 100 100"

echo "loop sets the repeat count"
request_file animated.gif "f=gif&anim=loop:3" "${out}-loop.gif"
check "loop" "$(meta "${out}-loop.gif[n=-1]" loop)" "3"

echo "stride keeps every Nth frame"
request_file animated.gif "f=gif&anim=stride:2" "${out}-stride.gif"
check "frame count" "$(meta "${out}-stride.gif[n=-1]" n-pages)" "2"
vips copy "${out}-stride.gif[n=-1]" "${out}-stride.v"
assert_pixel "first kept frame is white" "${out}-stride.v" 50 50 255 255 255
assert_pixel "second kept frame is green" "${out}-stride.v" 50 150 0 255 0

echo "frames caps the sequence"
request_file animated.gif "f=gif&anim=frames:2" "${out}-frames.gif"
check "frame count" "$(meta "${out}-frames.gif[n=-1]" n-pages)" "2"

echo "pages picks the frame the animation starts at"
request_file animated.gif "f=gif&pages=3" "${out}-start.gif"
check "frame count" "$(meta "${out}-start.gif[n=-1]" n-pages)" "2"
vips copy "${out}-start.gif[n=-1]" "${out}-start.v"
assert_pixel "starts at the third frame" "${out}-start.v" 50 50 0 255 0

echo "anim=off flattens the animation"
request_file animated.gif "f=gif&anim=off" "${out}-off.gif"
check "frame count" "$(meta "${out}-off.gif[n=-1]" n-pages)" "1"

echo "a still source is unaffected"
request_file still.png "f=gif" "${out}-still.gif"
check "frame count" "$(meta "${out}-still.gif[n=-1]" n-pages)" "1"

echo "a video is a single frame unless a clip is asked for"
request_file clip.mp4 "f=gif" "${out}-video-still.gif"
check "frame count" "$(meta "${out}-video-still.gif[n=-1]" n-pages)" "1"

echo "anim turns a video into a clip"
request_file clip.mp4 "f=gif&anim=frames:8|fps:8" "${out}-video.gif"
check "frame count" "$(meta "${out}-video.gif[n=-1]" n-pages)" "8"
# GIF stores a delay in centiseconds, so 125 ms comes back as 120. WebP keeps
# the millisecond the request asked for.
request_file clip.mp4 "f=webp&anim=frames:8|fps:8" "${out}-video.webp"
check "frame count" "$(meta "${out}-video.webp[n=-1]" n-pages)" "8"
check "delays" "$(meta "${out}-video.webp[n=-1]" delay | cut -d' ' -f1)" "125"

echo "a stride thins a video clip out without shortening it"
# The stride is folded into the rate ffmpeg samples at, so it must not also be
# applied to the frames that come back: 6 were asked for, 6 have to arrive.
request_file clip.mp4 "f=webp&anim=frames:6|fps:8|stride:2" "${out}-video-stride.webp"
check "frame count" "$(meta "${out}-video-stride.webp[n=-1]" n-pages)" "6"
check "delays" "$(meta "${out}-video-stride.webp[n=-1]" delay | cut -d' ' -f1)" "125"

echo "widely spaced samples are seeked to, not decoded up to"
# One sample every ten seconds, off a video whose colour says which second a
# frame came from: 5s red, 15s green, 25s blue, 35s white.
request_file blocks.mp4 "f=webp&anim=frames:4|fps:1|stride:10&t=5" "${out}-video-sparse.webp"
check "frame count" "$(meta "${out}-video-sparse.webp[n=-1]" n-pages)" "4"
check "delays" "$(meta "${out}-video-sparse.webp[n=-1]" delay | cut -d' ' -f1)" "1000"
vips copy "${out}-video-sparse.webp[n=-1]" "${out}-video-sparse.v"
assert_pixel "samples 5s"  "${out}-video-sparse.v" 50 50  255 0 0
assert_pixel "samples 15s" "${out}-video-sparse.v" 50 150 0 255 0
assert_pixel "samples 25s" "${out}-video-sparse.v" 50 250 0 0 255
assert_pixel "samples 35s" "${out}-video-sparse.v" 50 350 255 255 255

echo "a clip stops where the video ends"
# Samples at 25s and 35s land inside the video, 45s and beyond do not.
request_file blocks.mp4 "f=webp&anim=frames:6|fps:1|stride:10&t=25" "${out}-video-short.webp"
check "frame count" "$(meta "${out}-video-short.webp[n=-1]" n-pages)" "2"

echo "a video ignores pages and is addressed by time"
request_file blocks.mp4 "f=webp&pages=3&anim=frames:4|fps:1|stride:10&t=5" "${out}-video-pages.webp"
check "frame count" "$(meta "${out}-video-pages.webp[n=-1]" n-pages)" "4"
vips copy "${out}-video-pages.webp[n=-1]" "${out}-video-pages.v"
assert_pixel "still starts at 5s" "${out}-video-pages.v" 50 50 255 0 0

echo "a clip and a still frame of the same video do not share a cache entry"
request_file clip.mp4 "f=gif" "${out}-video-still2.gif"
check "still is still a still" "$(meta "${out}-video-still2.gif[n=-1]" n-pages)" "1"

echo "a clip is sized like any other animation"
request_file clip.mp4 "f=webp&anim=frames:6|fps:6&w=50&h=50&fit=cover" "${out}-video-small.webp"
check "frame count" "$(meta "${out}-video-small.webp[n=-1]" n-pages)" "6"
check "frame height" "$(meta "${out}-video-small.webp[n=-1]" page-height)" "50"

echo "a video clip in a format that cannot animate stays a single frame"
request_file clip.mp4 "f=jpeg&anim=frames:8" "${out}-video-flat.jpg"
if vips getpoint "${out}-video-flat.jpg" 0 100 >/dev/null 2>&1; then
    echo "  FAIL jpeg clip kept more than one frame"
    failures=$((failures + 1))
else
    echo "  ok   jpeg clip is a single frame"
fi

echo "a size limit does not break the frame count"
request_file animated.gif "f=gif&limit=size:4K" "${out}-limit.gif"
check "frame count" "$(meta "${out}-limit.gif[n=-1]" n-pages)" "4"

if [[ "${failures}" -ne 0 ]]; then
    echo "${failures} check(s) failed"
    exit 1
fi

echo "all animation checks passed"
