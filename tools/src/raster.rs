use crate::{Assets, Manifest, Result, read_asset, validate_png, validate_source};
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Frames {
    pub version: u32,
    pub source_sha256: String,
    pub duration_ms: u32,
    pub fps: u32,
    pub frame_size: [u32; 2],
    pub frame_count: u32,
    pub columns: u32,
    pub rows: u32,
    pub background_rgba: [u8; 4],
    pub pages: Vec<String>,
}

#[derive(Deserialize)]
struct Rendered {
    pages: Vec<String>,
    background_rgba: [u8; 4],
}

pub fn bake(root: &Path, output: &Path) -> Result<()> {
    validate_source(root)?;
    let root = root.canonicalize()?;
    let source = read_asset(&root, Path::new("splash/source.svg"))?;
    let manifest: Manifest =
        serde_json::from_slice(&read_asset(&root, Path::new("splash/manifest.json"))?)?;
    let frame_count = (manifest.motion_end_ms * 60).div_ceil(1000) + 1;
    let settings = serde_json::json!({
        "frame_size": [480, 400], "fps": 60, "columns": 4, "rows": 2,
        "frame_count": frame_count, "animation_count": manifest.animations.len(),
    });
    let html = include_str!("raster.html")
        .replace("__SOURCE__", std::str::from_utf8(&source)?)
        .replace("__SETTINGS__", &settings.to_string());
    fs::create_dir(output)?;
    let result = (|| -> Result<()> {
        let work = output.join(".render");
        fs::create_dir(&work)?;
        fs::create_dir(work.join("profile"))?;
        fs::write(work.join("index.html"), html)?;
        let stdout = fs::File::create(work.join("stdout"))?;
        let stderr = fs::File::create(work.join("stderr"))?;
        let chromium = std::env::var_os("CHROMIUM").unwrap_or_else(|| "chromium".into());
        let mut child = Command::new(chromium)
            .args([
                "--headless",
                "--no-sandbox",
                "--disable-gpu",
                "--disable-background-networking",
                "--no-first-run",
                "--virtual-time-budget=20000",
                "--dump-dom",
            ])
            .arg(format!(
                "--user-data-dir={}",
                work.join("profile").canonicalize()?.display()
            ))
            .arg(format!(
                "file://{}",
                work.join("index.html").canonicalize()?.display()
            ))
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()?;
        let mut status = None;
        for _ in 0..1200 {
            status = child.try_wait()?;
            if status.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let status = match status {
            Some(status) => status,
            None => {
                child.kill()?;
                child.wait()?;
                return Err("Chromium frame export timed out".into());
            }
        };
        if !status.success() {
            return Err("Chromium frame export failed".into());
        }
        let mut stdout = String::new();
        fs::File::open(work.join("stdout"))?
            .take(32 * 1024 * 1024 + 1)
            .read_to_string(&mut stdout)?;
        if stdout.len() > 32 * 1024 * 1024 {
            return Err("Frame export exceeded 32 MiB".into());
        }
        let json = stdout
            .split_once("<pre id=\"frames\">")
            .and_then(|(_, rest)| rest.split_once("</pre>"))
            .map(|(json, _)| json)
            .ok_or("Missing rendered frames")?;
        let rendered: Rendered = serde_json::from_str(json).map_err(|error| {
            format!(
                "Invalid rendered frames: {error}; {}",
                json.chars().take(300).collect::<String>()
            )
        })?;
        let page_count = frame_count.div_ceil(8) as usize;
        if rendered.pages.len() != page_count {
            return Err("Unexpected rendered page count".into());
        }
        let mut frames = Frames {
            version: 1,
            source_sha256: format!("{:x}", Sha256::digest(&source)),
            duration_ms: manifest.duration_ms,
            fps: 60,
            frame_size: [480, 400],
            frame_count,
            columns: 4,
            rows: 2,
            background_rgba: rendered.background_rgba,
            pages: Vec::with_capacity(page_count),
        };
        for (index, page) in rendered.pages.iter().enumerate() {
            let bytes = base64::engine::general_purpose::STANDARD.decode(page)?;
            validate_png(&bytes, [1920, 800])?;
            let name = format!("atlas-{index:03}.png");
            fs::write(output.join(&name), bytes)?;
            frames.pages.push(format!("splash/frames/{name}"));
        }
        fs::write(
            output.join("manifest.json"),
            serde_json::to_vec_pretty(&frames)?,
        )?;
        fs::remove_dir_all(work)?;
        Ok(())
    })();
    if let Err(error) = result {
        fs::remove_dir_all(output)
            .map_err(|cleanup| format!("{error}; cannot remove incomplete bake: {cleanup}"))?;
        return Err(error);
    }
    Ok(())
}

pub fn validate(root: &Path) -> Result<Assets> {
    let root = root.canonicalize()?;
    let bytes = read_asset(&root, Path::new("splash/frames/manifest.json"))?;
    let frames: Frames = serde_json::from_slice(&bytes)?;
    let source = read_asset(&root, Path::new("splash/source.svg"))?;
    let manifest: Manifest =
        serde_json::from_slice(&read_asset(&root, Path::new("splash/manifest.json"))?)?;
    if frames.source_sha256 != format!("{:x}", Sha256::digest(source)) {
        return Err("Baked frames are stale; run bake again".into());
    }
    if frames.version != 1
        || frames.fps != 60
        || frames.frame_size != [480, 400]
        || frames.columns != 4
        || frames.rows != 2
        || frames.background_rgba[3] != 255
        || frames.duration_ms != manifest.duration_ms
        || frames.frame_count != (manifest.motion_end_ms * 60).div_ceil(1000) + 1
        || frames.pages.len() != frames.frame_count.div_ceil(8) as usize
    {
        return Err("Invalid baked frame metadata".into());
    }
    let mut assets = vec![("splash/frames/manifest.json".into(), bytes)];
    for (index, page) in frames.pages.iter().enumerate() {
        if *page != format!("splash/frames/atlas-{index:03}.png") {
            return Err("Invalid atlas path".into());
        }
        let bytes = read_asset(&root, Path::new(page))?;
        validate_png(&bytes, [1920, 800])?;
        assets.push((page.into(), bytes));
    }
    Ok(assets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_pages_cover_motion_and_leave_a_final_hold() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let assets = validate(root).unwrap();
        assert_eq!(assets.len(), 11);
        let frames: Frames = serde_json::from_slice(&assets[0].1).unwrap();
        assert_eq!(frames.frame_count, 73);
        assert_eq!(frames.background_rgba, [37, 44, 36, 255]);
        assert!(frames.frame_count * 1000 / frames.fps < frames.duration_ms);
    }

    #[test]
    fn changing_the_source_requires_rebaking_the_exported_bundle() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let output = std::env::temp_dir().join(format!(
            "brand-frame-staleness-{}-{nonce}",
            std::process::id()
        ));
        crate::export(&crate::validate(source).unwrap(), &output).unwrap();
        assert!(crate::validate(&output).is_ok());
        let path = output.join("splash/source.svg");
        let mut svg = fs::read_to_string(&path).unwrap();
        svg.push('\n');
        fs::write(path, svg).unwrap();
        let error = crate::validate(&output).unwrap_err().to_string();
        fs::remove_dir_all(output).unwrap();
        assert!(error.contains("Baked frames are stale"));
    }
}
