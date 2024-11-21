# Picturium

_Fast and caching media server for processing images, generating thumbnails and serving files on the fly_

**!!! Early stages of development. Some features may not work properly and can change overtime without notice. !!!**


## Running picturium

picturium relies on `libvips` crate to provide libvips bindings. 
This means that the maximum currently supported version of libvips is `8.15.1`. 
Since building libvips while keeping system packages not broken is quite a challenge, it is recommended running picturium through Docker. 
There are 3 Docker images:

### picturium

This image contains ready-to-deploy picturium server with everything you are going to need. 
Replace `{picturium-data}` with local folder containing your `.env` file `data` directory containing files you want to serve. 
Make sure `picturium` user with UID/GID 1500 has write permissions to this folder (not necessarily your data directories).

```bash
docker run --rm -v {picturium-data}:/app -ti --init -p 20045:20045 lamka02sk/picturium:latest
```

### picturium-dev

Image to make development of picturium itself easier. Automatically watches for code changes and recompiles picturium. 
Simply run with bash script `dev.sh` in project root.

```bash
./dev.sh
```

### picturium-base

Base picturium image providing `libvips` and other necessary libraries for the final build. 
This image is used only as base for other images.

## Supported file formats

Supports all file formats in pass-through mode, but some of them get special treatment:

### Input formats

- JPG, JPEG, PNG, WEBP, SVG, TIF, TIFF, GIF, BMP, ICO
- HEIC, HEIF, JP2, JPM, JPX, JPF, AVIF, AVIFS
[//]: # (- ARW, RAW)
- PDF (for thumbnail generation or pass-through)
- DOC, DOCX, ODT, RTF (for thumbnail generation or pass-through)
- XLS, XLSX, ODS (for thumbnail generation or pass-through)
- PPT, PPTX, ODP (for thumbnail generation or pass-through)
- MP4, MKV, WEBM, AVI, MOV, FLV, WMV, MPG, MPEG, 3GP, OGV, M4V (for thumbnail generation using `mpv`)

### Output formats

- PDF (supported for office document files only)
- AVIF (served by default to all browsers supporting it, can be disabled by setting `AVIF_ENABLE` environment variable to `false`)
- WEBP (served to all browsers not supporting AVIF, or when AVIF is disabled)
- JPEG (served to all browsers not supporting AVIF and WEBP)
- PNG (served only when requested by the client)


## Caching

- automatically checks file creation, modification and last accessed time
- set maximum cache size on disk with environment variable `CACHE_CAPACITY` in GB
- old cached files are periodically purged from disk


## Serving files

All files are served from the working directory. The working directory in docker images is located at `/app`.\
For example file located at `/app/data/image.jpeg` will be available at `https://.../data/image.jpeg`.


## Token authorization

- by default, picturium **requires** token authorization of all requests to protect against unwanted traffic
- you can disable token authorization by completely removing `KEY` environment variable from `.env` file
- tokens are SHA256 HMAC authentication codes
- token is generated from file path + all URL parameters except the `token` parameter, sorted alphabetically (check out `RawUrlParameters::verify_token` in [src/parameters/mod.rs](https://github.com/lamka02sk/picturium/blob/master/src/parameters/mod.rs) for more)

- [How to generate token with PHP](examples/generate_token.php)


## URL GET parameters

- [x] `w` (int): width of the output image in pixels
- [x] `h` (int): height of the output image in pixels
- [ ] `ar` (string): aspect ratio of the output image, when both `w` and `h` are set, this parameter will be ignored
  - `auto` (default): aspect ratio will be set by `w` and `h` parameters, or original image dimensions if not both `w` and `h` are set 
  - `video`: ratio 16/9
  - `square`: ratio 1/1
  - custom aspect ratio like `4/3`, `16/10`, `3/2`
- [x] `q` (int): quality of the output image in percent (default: dynamic quality based on the requested image dimensions)
- [x] `dpr` (int): device pixel ratio, multiplies `w` and `h` by itself
- [ ] `crop` (string): crop parameters in format `crop=ar:auto,w:50,h:50,g:center,x:0,y:0`; for cropping the image, at least one of `w` or `h` must be set
    - `ar`: aspect ratio of the crop area
        - `auto` (default): aspect ratio will be set by `w` and `h` crop parameters, or original image dimensions if not both `w` and `h` are set
        - `video`: ratio 16/9
        - `square`: ratio 1/1
        - custom aspect ratio like `4/3`, `16/10`, `3/2`
    - `w`: width of the crop area in pixels relative to the original image size
    - `h`: height of the crop area in pixels relative to the original image size
    - `g`: gravity / placement of the cropped area within the original image, default: `center`
        - `center`: center of the original image
        - `top-left`|`left-top`: left top corner of the original image
        - `top-center`|`center-top`|`top`: top center of the original image
        - `top-right`|`right-top`: right top corner of the original image
        - `left-center`|`center-left`|`left`: left center of the original image
        - `right-center`|`center-right`|`right`: right center of the original image
        - `bottom-left`|`left-bottom`: left bottom corner of the original image
        - `bottom-center`|`center-bottom`|`bottom`: bottom center of the original image
        - `bottom-right`|`right-bottom`: right bottom corner of the original image
    - `x`: offset on the X axis (horizontal) in pixels from the center of gravity, negative values are supported
    - `y`: offset on the Y axis (vertical) in pixels from the center of gravity, negative values are supported
- [x] `thumb`: generate thumbnail from file, or specific page of PDF documents in format `thumb=p:1`
    - `p`: page of the document to generate thumbnail, default: `1`
- [x] `original`: default `false`
    - `true`: returns original image or file, all other URL parameters are ignored
    - `false`: returns processed image
- [x] `rot`: rotate image, default: `no`
    - `90`|`left`|`anticlockwise`: rotate image left by 90 degrees
    - `180`|`bottom-up`|`upside-down`: rotate image upside down by 180 degrees
    - `270`|`right`|`clockwise`: rotate image right by 90 degrees
- [x] `bg`: apply background color to transparent image, colors can be specified in different formats:
    - HEX (e.g. `#ffffff`, `#7a7ad3`, `#000000ff`)
    - RGB (e.g. `255,124,64`)
    - RGBA (e.g. `255,124,64,255`)
    - predefined value (`transparent`|`black`|`white`)
- [x] `f`: specify output format, default: `auto`
    - `auto`: automatically selects the best format for the requesting web browser
    - `jpg`|`jpeg`: output image in JPEG format
    - `webp`: output image in WEBP format
    - `avif`: output image in AVIF format
    - `png`: output image in PNG format
    - `pdf`: output office document in PDF format / defaults to JPEG for images and PDF files


### Example URL

The original image will be processed, rotated left by 90 degrees, resized to be 320px wide while keeping the original aspect ratio, saved with 50% quality in a format (WEBP or JPEG) supported by the requesting web browser.

```url
https://example.com/folder/test.jpg?token=fsd5f4sd5f4&w=160&q=50&dpr=2&rot=left
```

## Limitations

picturium uses a few libraries that enforce limits on the size of images that can be processed. 
We tried to discover and tailor these limits to ensure stability and good (not only) developer experience.

### PNG
Maximum output image resolution: `16384 x 16384 px` (reason: quantization)

### WebP
Maximum output image resolution: `16383 x 16383 px` (reason: WebP format limitation)\   
Maximum total output image resolution: `170 megapixels` (reason: `cwebp` internal limitations)

### AVIF
Maximum output image resolution: `16384 x 16384 px` (reason: `libvips` internal limitation)

### SVG
Images included in SVG files (`xlink:href`), cannot exceed memory limit of 512 MB 
(https://gitlab.gnome.org/GNOME/librsvg/-/issues/1093) due to default configuration of `image` crate
which cannot be increased through both `librsvg` and `libvips`.

According to test files found in `image` crate, the memory needed to process the image (with reserve) can be calculated like this:

```
{image_width} * {image_height} * 5 / 1024 / 1024
```

We recommend including images with maximum resolution of `105 megapixels` (or for example `10000 x 10500 px`).
