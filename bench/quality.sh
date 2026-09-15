#!/usr/bin/env bash
# Quality helpers, run inside the picturium dev image (has vips and ffmpeg).
#
#   quality.sh ref   <source> <width> <height> <out.png>
#   quality.sh score <ref.png> <candidate>   -> "<ssim> <psnr> <width>x<height>"
#   quality.sh dims  <file>                  -> "<width>x<height>"
#   quality.sh clean                         empty bench/out
set -euo pipefail

metric() {
    local filter="$1" key="$2" a="$3" b="$4"
    ffmpeg -nostdin -v error -i "$a" -i "$b" \
        -lavfi "[0:v]format=yuv444p[a];[1:v]format=yuv444p[b];[a][b]${filter}=stats_file=-" \
        -f null - | sed -n "s/.*${key}:\([0-9.a-z]*\).*/\1/p" | head -n1
}

# vips flatten rejects an image that has no alpha channel to composite, so an
# image without one is passed through unchanged.
flatten() {
    vips flatten "$1" "$2" --background 255,255,255 2>/dev/null \
        || vips copy "$1" "$2"
}

dimensions() {
    ffprobe -v error -select_streams v -show_entries stream=width,height -of csv=s=x:p=0 "$1"
}

case "${1:-}" in
ref)
    # Reference render: same crop and box as the servers, lanczos3, lossless.
    vips thumbnail "$2" "$5" "$3" --height "$4" --crop centre
    ;;
clean)
    # Only the contents: the directory itself is created by the host, and
    # recreating it here would leave it owned by root and unwritable there.
    mkdir -p bench/out
    rm -rf bench/out/..?* bench/out/.[!.]* bench/out/*
    ;;
dims)
    dimensions "$2"
    ;;
score)
    tmp="$(mktemp -d)"
    trap 'rm -rf "${tmp}"' EXIT
    # Composite both onto the same opaque background first. ffmpeg compares RGB
    # and ignores alpha, so a lossy encoder's arbitrary colour under fully
    # transparent pixels would otherwise dominate the score even though nothing
    # there is visible.
    flatten "$3" "${tmp}/cand.png"
    flatten "$2" "${tmp}/ref.png"
    size="$(dimensions "${tmp}/cand.png")"
    if [[ "${size}" != "$(dimensions "${tmp}/ref.png")" ]]; then
        echo "n/a n/a ${size}"
        exit 0
    fi
    echo "$(metric ssim All "${tmp}/cand.png" "${tmp}/ref.png") $(metric psnr psnr_avg "${tmp}/cand.png" "${tmp}/ref.png") ${size}"
    ;;
*)
    echo "usage: quality.sh ref|score|dims|clean ..." >&2
    exit 2
    ;;
esac
