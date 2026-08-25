use crate::enums::dpi::Dpi;
use crate::process::pipeline::request::PipelineRequest;
use crate::process::source::Source;
use crate::services::cache::path_generator::generate_intermediate_path;
use crate::services::cache::sidecar;
use anyhow::{Context, Result, anyhow};
use krilla::Document;
use krilla::geom::Size;
use krilla::page::PageSettings;
use krilla_svg::{SurfaceExt, SvgSettings};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use usvg::fontdb;

static FONTS: LazyLock<Arc<fontdb::Database>> = LazyLock::new(|| {
    let mut database = fontdb::Database::new();
    database.load_system_fonts();
    Arc::new(database)
});

/// Converts an SVG source into a single-page vector PDF
pub async fn process(request: &PipelineRequest<'_>) -> Result<String> {
    let source_path = &request.source.path;
    let style = request.parameters.style.clone();
    let resources = local_resources(request);

    let dpi = match request.parameters.dpi {
        Dpi::Auto => request.state.config.svg.load_dpi,
        Dpi::Value(value) => value as u32,
    };

    let variant = match &style {
        Some(style) => format!("{dpi}dpi-{}", hash_style(style)),
        None => format!("{dpi}dpi"),
    };

    let pdf_path = generate_intermediate_path(request, source_path, &format!("{variant}.pdf"));

    if sidecar::is_valid(&pdf_path, source_path).await {
        return Ok(pdf_path);
    }

    let data = tokio::fs::read(source_path).await?;
    let pdf =
        tokio::task::spawn_blocking(move || convert(&data, dpi as f32, style, resources)).await??;

    let pdf_dir = Path::new(&pdf_path)
        .parent()
        .with_context(|| "Invalid pdf path")?;

    tokio::fs::create_dir_all(pdf_dir).await?;
    save_pdf(&pdf_path, &pdf).await?;
    sidecar::write(&pdf_path, source_path).await?;

    Ok(pdf_path)
}

/// The directory an SVG may pull `<image href="...">` files from, and the data
/// directory those files must stay inside. `None` forbids file references entirely.
struct LocalResources {
    source_dir: PathBuf,
    data_dir: String,
}

fn local_resources(request: &PipelineRequest<'_>) -> Option<LocalResources> {
    if !request.state.config.svg.allow_local_resources {
        return None;
    }

    Some(LocalResources {
        source_dir: request.source.path.parent()?.to_path_buf(),
        data_dir: request.state.config.data.dir.clone(),
    })
}

/// Resolve files referenced in SVG against allowed resource paths
fn resolve_href(resources: Option<LocalResources>) -> usvg::ImageHrefStringResolverFn<'static> {
    let Some(resources) = resources else {
        return Box::new(|href, _| {
            tracing::debug!("Refusing to load \"{href}\" referenced by an SVG: svg.allow_local_resources is off");
            None
        });
    };

    let default = usvg::ImageHrefResolver::default_string_resolver();

    Box::new(move |href, options| {
        let candidate = resources.source_dir.join(href);

        let Some(path) = Source::get_path(&candidate.to_string_lossy(), &resources.data_dir) else {
            tracing::debug!("Refusing to load \"{href}\" referenced by an SVG: outside the data directory");
            return None;
        };

        default(&path.to_string_lossy(), options)
    })
}

fn convert(
    data: &[u8],
    dpi: f32,
    style: Option<String>,
    resources: Option<LocalResources>,
) -> Result<Vec<u8>> {
    let options = usvg::Options {
        dpi,
        fontdb: FONTS.clone(),
        style_sheet: style,
        resources_dir: None,
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: usvg::ImageHrefResolver::default_data_resolver(),
            resolve_string: resolve_href(resources),
        },
        ..Default::default()
    };

    let tree = usvg::Tree::from_data(data, &options)?;

    let scale = 72.0 / dpi;
    let size = Size::from_wh(tree.size().width() * scale, tree.size().height() * scale)
        .with_context(|| "SVG has no usable size")?;

    let mut document = Document::new();
    let mut page = document.start_page_with(PageSettings::new(size));
    let mut surface = page.surface();

    surface
        .draw_svg(&tree, size, SvgSettings::default())
        .with_context(|| "Failed to draw SVG onto the PDF page")?;

    surface.finish();
    page.finish();

    document.finish().map_err(|e| anyhow!("{e:?}"))
}

fn hash_style(style: &str) -> String {
    let mut hasher = DefaultHasher::new();
    style.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

async fn save_pdf(pdf_path: &str, pdf: &[u8]) -> Result<()> {
    let temporary_path = format!("{pdf_path}.{}.tmp", std::process::id());

    tokio::fs::write(&temporary_path, pdf).await?;
    tokio::fs::rename(&temporary_path, pdf_path).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{LocalResources, convert};
    use lopdf::{Document, Object};

    const SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="144" height="72"><rect width="144" height="72" fill="blue"/></svg>"#;

    fn media_box(pdf: &[u8]) -> Vec<f32> {
        let document = Document::load_mem(pdf).unwrap();
        let (_, page_id) = document.get_pages().into_iter().next().unwrap();

        document
            .get_dictionary(page_id)
            .unwrap()
            .get(b"MediaBox")
            .and_then(Object::as_array)
            .unwrap()
            .iter()
            .map(|value| value.as_float().unwrap())
            .collect()
    }

    #[test]
    fn a_72_dpi_page_is_one_point_per_svg_pixel() {
        let pdf = convert(SVG.as_bytes(), 72.0, None, None).unwrap();

        assert!(pdf.starts_with(b"%PDF"));
        assert_eq!(media_box(&pdf), vec![0.0, 0.0, 144.0, 72.0]);
    }

    #[test]
    fn a_higher_dpi_makes_the_same_svg_a_smaller_page() {
        let pdf = convert(SVG.as_bytes(), 144.0, None, None).unwrap();
        assert_eq!(media_box(&pdf), vec![0.0, 0.0, 72.0, 36.0]);
    }

    #[test]
    fn a_stylesheet_changes_the_output() {
        let plain = convert(SVG.as_bytes(), 72.0, None, None).unwrap();
        let styled = convert(SVG.as_bytes(), 72.0, Some("rect { fill: red }".into()), None).unwrap();

        assert_ne!(plain, styled);
    }

    /// A 1x1 PNG, the smallest thing usvg will accept as a referenced image.
    const PNG: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8,
        0xcf, 0xc0, 0x00, 0x00, 0x03, 0x01, 0x01, 0x00, 0xc9, 0xfe, 0x92, 0xef, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82
    ];

    fn svg_referencing(href: &str) -> String {
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="10" height="10"><image xlink:href="{href}" width="10" height="10"/></svg>"#
        )
    }

    /// usvg drops an `<image>` it cannot resolve, so an image XObject in the PDF
    /// means the reference was followed. The file's own bytes are not searched for,
    /// because krilla re-encodes the pixels on the way in.
    fn embeds_an_image(pdf: &[u8]) -> bool {
        Document::load_mem(pdf)
            .unwrap()
            .objects
            .values()
            .filter_map(|object| object.as_stream().ok())
            .any(|stream| {
                stream.dict.get(b"Subtype").and_then(Object::as_name).is_ok_and(|name| name == b"Image")
            })
    }

    struct Fixture {
        _root: tempfile::TempDir,
        data_dir: String,
        source_dir: std::path::PathBuf,
        outside: std::path::PathBuf,
    }

    /// A data directory holding `sibling.png`, and a `secret.png` next to it but
    /// outside the data directory entirely.
    fn fixture() -> Fixture {
        let root = tempfile::TempDir::new().unwrap();
        let data_dir = root.path().join("data");

        std::fs::create_dir(&data_dir).unwrap();
        std::fs::write(data_dir.join("sibling.png"), PNG).unwrap();

        let outside = root.path().join("secret.png");
        std::fs::write(&outside, PNG).unwrap();

        Fixture {
            data_dir: data_dir.to_string_lossy().into_owned(),
            source_dir: data_dir,
            outside,
            _root: root,
        }
    }

    fn resources(fixture: &Fixture) -> LocalResources {
        LocalResources {
            source_dir: fixture.source_dir.clone(),
            data_dir: fixture.data_dir.clone(),
        }
    }

    #[test]
    fn a_referenced_file_is_ignored_unless_local_resources_are_allowed() {
        let fixture = fixture();
        let svg = svg_referencing("sibling.png");

        let pdf = convert(svg.as_bytes(), 72.0, None, None).unwrap();

        assert!(!embeds_an_image(&pdf));
    }

    #[test]
    fn a_sibling_file_is_embedded_when_local_resources_are_allowed() {
        let fixture = fixture();
        let svg = svg_referencing("sibling.png");

        let pdf = convert(svg.as_bytes(), 72.0, None, Some(resources(&fixture))).unwrap();

        assert!(embeds_an_image(&pdf));
    }

    #[test]
    fn an_absolute_path_outside_the_data_directory_is_refused() {
        let fixture = fixture();
        let svg = svg_referencing(&fixture.outside.to_string_lossy());

        let pdf = convert(svg.as_bytes(), 72.0, None, Some(resources(&fixture))).unwrap();

        assert!(!embeds_an_image(&pdf));
    }

    #[test]
    fn a_traversing_relative_path_is_refused() {
        let fixture = fixture();
        let svg = svg_referencing("../secret.png");

        let pdf = convert(svg.as_bytes(), 72.0, None, Some(resources(&fixture))).unwrap();

        assert!(!embeds_an_image(&pdf));
    }
}
