# Picturium

_Fast and caching media server for processing images, generating thumbnails and serving files on the fly_

**!!! Early stages of development. Some features may not work properly and can change overtime without notice. !!!**


## Running picturium

_picturium_ relies on `libvips` crate to provide libvips bindings. This means that the maximum currently supported version of libvips is `8.15.1`. Since building libvips while keeping system packages not broken is quite a challenge, it is recommended running picturium through Docker. There are 3 Docker images:

### picturium

This image contains ready-to-deploy picturium server with everything you are going to need. Replace `{picturium-data}` with local folder containing your `.env` file `data` directory containing files you want to serve. Make sure `picturium` user with UID/GID 1500 has write permissions to this folder (not necessarily your data directories).

```bash
docker run --rm -v {picturium-data}:/app -ti --init -p 20045:20045 lamka02sk/picturium:latest
```

### picturium-dev

Image to make development of picturium itself easier. Automatically watches for code changes and recompiles picturium.

```bash
docker run --rm -v {picturium-git}:/root/picturium -it --init -p 20045:20045 lamka02sk/picturium-dev:8.15.1
```

### picturium-base

Base picturium image providing `libvips` and other necessary libraries for the final build. This image is used only as base for other images.


## Caching

- automatically checks file creation, modification and last accessed time
- set maximum cache size on disk with environment variable `CACHE_CAPACITY` in GB
- old cached files are periodically purged from disk


## Token authorization

- _picturium_ supports token authorization of requests to protect against bots or other unwanted traffic
- if environment variable `KEY` is not set, token authorization will be disabled, otherwise each request needs to be signed with SHA256 HMAC token
- token is generated from file path + all URL parameters except `token` parameter, sorted alphabetically (check out `RawUrlParameters::verify_token` in [src/parameters/mod.rs](https://github.com/lamka02sk/picturium/blob/master/src/parameters/mod.rs) for more)


## URL GET parameters

- `w` (int): width of the output image in pixels
- `h` (int): height of the output image in pixels
- `q` (int): quality of the output image in percent (default: JPEG - 75%, WEBP - 70%)
- `dpr` (int): device pixel ratio, multiplies `w` and `h` by itself
- `crop` (string): crop parameters in format `crop=aspect_ratio,width,height,gravity,offset_x,offset_y`
    - `aspect_ratio`:
        - `video`: ratio 16:9
        - `square`: ratio 1:1
        - `free`: aspect ratio will be set by `width` and `height` aspect ratio parameters
        - or just use your own like this `4:3`, `16:10`, `3:2`
    - `width`: width of the crop area in pixels relative to the original image size; when `0`, this parameter will be ignored
    - `height`: height of the crop area in pixels relative to the original image size; when `0`, this parameter will be ignored
    - `gravity`: placement of the cropped area within the original image, default: `center`
        - `top-left`|`left-top`: left top corner of the original image
        - `top-center`|`center-top`|`top`: top center of the original image
        - `top-right`|`right-top`: right top corner of the original image
        - `left-center`|`center-left`|`left`: left center of the original image
        - `right-center`|`center-right`|`right`: right center of the original image
        - `bottom-left`|`left-bottom`: left bottom corner of the original image
        - `bottom-center`|`center-bottom`|`bottom`: bottom center of the original image
        - `bottom-right`|`right-bottom`: right bottom corner of the original image
        - any other value will be processed like `center`
    - `offset_x`: offset on the X axis (horizontal) in pixels from the center of gravity, negative values are supported
    - `offset_y`: offset on the Y axis (vertical) in pixels from the center of gravity, negative values are supported
- `thumb`: generate thumbnail from file, or specific page of PDF documents in format `thumb=page`
    - `page`: page of the document to generate thumbnail, default: `1`
- `original`: default `false`
    - `true`: returns original image or file, all other URL parameters are ignored
    - `false`: returns processed image
- `rot`: rotate image, default: `no`
    - `90`|`left`|`anticlockwise`: rotate image left by 90 degrees
    - `180`|`bottom-up`|`upside-down`: rotate image upside down by 180 degrees
    - `270`|`right`|`clockwise`: rotate image right by 90 degrees
- `bg`: apply background color to transparent image, colors can be specified in different formats:
    - hexadecimal format (e.g. `#ffffff`, `#7a7ad3`, `#000000ff`)
    - RGB matrix (e.g. `255,124,64`)
    - RGBA matrix (e.g. `255,124,64,255`)
    - predefined value (`transparent`|`black`|`white`)


### Example URL

The original image will be processed, rotated left by 90 degrees, resized to be 320px wide while keeping the original aspect ratio, saved with 50% quality in a format (WEBP or JPEG) supported by the requesting web browser.

```url
https://example.com/folder/test.jpg?token=fsd5f4sd5f4&w=160&q=50&dpr=2&rot=left
```
