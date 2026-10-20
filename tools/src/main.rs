use serde::Deserialize;
use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
type Assets = Vec<(PathBuf, Vec<u8>)>;

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("brand-assets: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    duration_ms: u32,
    motion_end_ms: u32,
    background: String,
    view_box: [u32; 4],
    source: String,
    poster: String,
    fallback: String,
    fallback_size: [u32; 2],
    logos: Vec<String>,
    animations: Vec<Animation>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Animation {
    selector: String,
    name: String,
    duration_ms: u32,
}

fn run() -> Result<()> {
    let arguments: Vec<String> = std::env::args().take(5).collect();
    match arguments.get(1).map(String::as_str) {
        Some("validate") if (2..=3).contains(&arguments.len()) => {
            let root = arguments.get(2).map_or(Path::new("."), Path::new);
            let assets = validate(root)?;
            println!("Validated {} files", assets.len());
        }
        Some("export") if (3..=4).contains(&arguments.len()) => {
            let root = arguments.get(3).map_or(Path::new("."), Path::new);
            let assets = validate(root)?;
            export(&assets, Path::new(&arguments[2]))?;
            println!("Exported {} files to {}", assets.len(), arguments[2]);
        }
        _ => return Err("usage: brand-assets validate [ROOT] | export OUTPUT [ROOT]".into()),
    }
    Ok(())
}

fn read_asset(root: &Path, relative: &Path) -> Result<Vec<u8>> {
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(format!("invalid asset path: {}", relative.display()).into());
    }
    let path = root.join(relative).canonicalize()?;
    if !path.starts_with(root) {
        return Err(format!("asset escapes repository: {}", relative.display()).into());
    }
    let mut bytes = Vec::new();
    fs::File::open(&path)?
        .take(2_097_153)
        .read_to_end(&mut bytes)?;
    if bytes.is_empty() || bytes.len() > 2_097_152 {
        return Err(format!("asset must contain 1 byte to 2 MiB: {}", relative.display()).into());
    }
    Ok(bytes)
}

fn validate(root: &Path) -> Result<Assets> {
    let root = root.canonicalize()?;
    let manifest_bytes = read_asset(&root, Path::new("splash/manifest.json"))?;
    let manifest: Manifest = serde_json::from_slice(&manifest_bytes)?;
    if manifest.version != 1 {
        return Err("unsupported manifest version".into());
    }
    if !(1..=10_000).contains(&manifest.duration_ms)
        || !(1..=manifest.duration_ms).contains(&manifest.motion_end_ms)
    {
        return Err("invalid splash duration".into());
    }
    if manifest.view_box[2..]
        .iter()
        .any(|value| !(1..=4096).contains(value))
    {
        return Err("invalid SVG canvas size".into());
    }
    if manifest
        .fallback_size
        .iter()
        .any(|value| !(1..=8192).contains(value))
    {
        return Err("invalid PNG canvas size".into());
    }
    if u64::from(manifest.view_box[2]) * u64::from(manifest.fallback_size[1])
        != u64::from(manifest.view_box[3]) * u64::from(manifest.fallback_size[0])
    {
        return Err("SVG and PNG aspect ratios differ".into());
    }
    if manifest.logos.len() > 16 || !(1..=16).contains(&manifest.animations.len()) {
        return Err("too many logos or invalid animation count".into());
    }
    let source = read_asset(&root, Path::new(&manifest.source))?;
    let source_text = std::str::from_utf8(&source)?;
    let source_svg = validate_svg(source_text, Some(manifest.view_box))?;
    validate_animations(&manifest, &source_svg)?;
    let poster = read_asset(&root, Path::new(&manifest.poster))?;
    let poster_svg = validate_svg(std::str::from_utf8(&poster)?, Some(manifest.view_box))?;
    if poster_svg
        .descendants()
        .any(|node| node.has_tag_name("style") || node.has_tag_name("animate"))
    {
        return Err("poster must be static".into());
    }
    let fallback = read_asset(&root, Path::new(&manifest.fallback))?;
    validate_png(&fallback, manifest.fallback_size)?;
    for logo in &manifest.logos {
        let bytes = read_asset(&root, Path::new(logo))?;
        validate_svg(std::str::from_utf8(&bytes)?, None)?;
    }
    let paths = [
        "splash/manifest.json",
        "splash/preview.html",
        "LICENSE",
        "README.md",
        &manifest.source,
        &manifest.poster,
        &manifest.fallback,
    ];
    let mut names = BTreeSet::new();
    let mut assets = Vec::new();
    for name in paths
        .into_iter()
        .chain(manifest.logos.iter().map(String::as_str))
    {
        if !names.insert(name) {
            return Err(format!("duplicate asset path: {name}").into());
        }
        assets.push((PathBuf::from(name), read_asset(&root, Path::new(name))?));
    }
    Ok(assets)
}

fn validate_svg(text: &str, view_box: Option<[u32; 4]>) -> Result<roxmltree::Document<'_>> {
    let document = roxmltree::Document::parse(text)?;
    let root = document.root_element();
    if !root.has_tag_name(("http://www.w3.org/2000/svg", "svg")) {
        return Err("expected an SVG document".into());
    }
    if let Some(expected) = view_box {
        let actual: Vec<u32> = root
            .attribute("viewBox")
            .ok_or("missing SVG viewBox")?
            .split_whitespace()
            .take(5)
            .map(str::parse)
            .collect::<std::result::Result<_, _>>()?;
        if actual != expected {
            return Err("SVG viewBox differs from manifest".into());
        }
    }
    Ok(document)
}

fn validate_animations(manifest: &Manifest, document: &roxmltree::Document<'_>) -> Result<()> {
    let style = document
        .descendants()
        .find(|node| node.has_tag_name("style"))
        .and_then(|node| node.text())
        .ok_or("missing animation stylesheet")?;
    let style = style.split_whitespace().collect::<Vec<_>>().join(" ");
    if !style.contains(&format!("background: {}", manifest.background)) {
        return Err("background differs from manifest".into());
    }
    let animation_count = style
        .split("animation:")
        .skip(1)
        .filter(|declaration| !declaration.trim_start().starts_with("none;"))
        .count();
    if animation_count != manifest.animations.len() {
        return Err("animation count differs from manifest".into());
    }
    let mut selectors = BTreeSet::new();
    let mut motion_end_ms = 0;
    for animation in &manifest.animations {
        if !selectors.insert(&animation.selector)
            || !(1..=manifest.motion_end_ms).contains(&animation.duration_ms)
        {
            return Err("duplicate selector or invalid animation duration".into());
        }
        let selector = format!(".{} {{", animation.selector);
        let rule = style
            .split_once(&selector)
            .and_then(|(_, remaining)| remaining.split_once('}'))
            .map(|(rule, _)| rule)
            .ok_or_else(|| format!("missing animation selector: {}", animation.selector))?;
        if !rule.contains(&format!(
            "animation: {} {}ms ",
            animation.name, animation.duration_ms
        )) {
            return Err(
                format!("animation timing differs from manifest: {}", animation.name).into(),
            );
        }
        if !style.contains(&format!("@keyframes {} {{", animation.name)) {
            return Err(format!("missing keyframes: {}", animation.name).into());
        }
        if !document.descendants().any(|node| {
            node.attribute("class").is_some_and(|classes| {
                classes
                    .split_whitespace()
                    .any(|class| class == animation.selector)
            })
        }) {
            return Err(format!("unused animation selector: {}", animation.selector).into());
        }
        motion_end_ms = motion_end_ms.max(animation.duration_ms);
    }
    if motion_end_ms != manifest.motion_end_ms {
        return Err("motion end differs from manifest".into());
    }
    Ok(())
}

fn validate_png(bytes: &[u8], expected: [u32; 2]) -> Result<()> {
    if bytes.len() < 24 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" || &bytes[8..16] != b"\0\0\0\rIHDR" {
        return Err("invalid PNG header".into());
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into()?);
    if [width, height] != expected {
        return Err("PNG canvas size differs from manifest".into());
    }
    Ok(())
}

fn export(assets: &Assets, output: &Path) -> Result<()> {
    fs::create_dir(output)?;
    let result = (|| -> Result<()> {
        for (relative, bytes) in assets {
            let path = output.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, bytes)?;
        }
        Ok(())
    })();
    if let Err(error) = result {
        fs::remove_dir_all(output)
            .map_err(|cleanup| format!("{error}; cannot remove incomplete export: {cleanup}"))?;
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approved_assets_validate() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        assert_eq!(validate(root).unwrap().len(), 9);
    }

    #[test]
    fn timing_drift_is_rejected() {
        let mut manifest: Manifest =
            serde_json::from_str(include_str!("../../splash/manifest.json")).unwrap();
        let document = validate_svg(include_str!("../../splash/source.svg"), None).unwrap();
        manifest.animations[0].duration_ms += 1;
        assert!(
            validate_animations(&manifest, &document)
                .unwrap_err()
                .to_string()
                .contains("timing differs")
        );
    }

    #[test]
    fn paths_cannot_escape_root() {
        assert!(read_asset(Path::new("."), Path::new("../LICENSE")).is_err());
        assert!(read_asset(Path::new("."), Path::new("/LICENSE")).is_err());
    }

    #[test]
    fn incorrect_png_dimensions_are_rejected() {
        assert!(validate_png(include_bytes!("../../splash/poster.png"), [1, 1]).is_err());
        assert!(validate_png(b"not a png", [1536, 1280]).is_err());
    }
}
