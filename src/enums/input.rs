use strum::EnumString;

#[derive(Debug, Clone, Copy, Default)]
pub enum InputFormat {
    Vips(VipsInputFormat),
    Office(OfficeInputFormat),
    Video(VideoInputFormat),
    #[default]
    Unsupported,
}

#[derive(Debug, Clone, Copy, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum VipsInputFormat {
    #[strum(serialize = "jpeg")]
    Jpeg,
    Jp2k,
    Png,
    Tiff,
    Ico,
    Gif,
    Webp,
    Heif,
    Jxl,
    Pdf,
    Svg,
    Ai,
    Eps,
    Cdr,
    Psd,
    Bmp,
    Raw,
}

#[derive(Debug, Clone, Copy)]
pub enum OfficeInputFormat {
    Doc,
    Ppt,
    Xls,
}

#[derive(Debug, Clone, Copy)]
pub enum VideoInputFormat {
    Mp4,
    Webm,
    Mkv,
    Avi,
    Av1,
    _3gp,
    M4v,
    Flv,
    Mov,
    Mpeg,
}
