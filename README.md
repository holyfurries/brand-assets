# Holy Furries brand assets

Shared studio logos and the startup splash for Crowns Adrift, Cold Orbit, and
Spice CAD.

- `splash/source.svg`: approved animated splash, including its timing and forest
  background.
- `splash/poster.svg`: transparent static composition.
- `splash/poster.png`: approved transparent fallback at 1536 by 1280, with fixed
  lettering.
- `splash/manifest.json`: paths relative to the repository root, canvas size,
  background, and animation durations. All animations start at time zero.
- `splash/frames/`: shared 60 fps PNG atlas pages and their playback manifest.
- `logos/`: fox-and-halo mark, favicon, and square avatar source/export.
- `tools/`: Rust validation and bundle export CLI.

## Preview

```console
nix develop
miniserve . --index preview.html --port 8081 --interfaces 127.0.0.1 --header Cache-Control:no-store
```

Open <http://localhost:8081/splash/>. Replay restarts the sequence. Escape,
Space, clicking the artwork, or Skip jumps to the final frame. Reduced motion
shows the finished logo immediately.

## Application use

The sequence lasts 1.5 seconds. Motion finishes at 1185 ms, followed by a static
hold. The head silhouette squashes independently of the eyes and nose. Each
animation ends when its own movement settles to avoid a late sharpness change.

Web applications can embed `source.svg` in an SVG-capable browser context. The
preview demonstrates replay and skip handling. Native applications and game
engines should use the baked PNG atlases. These contain the browser-rendered
animation, including its easing, clipping, and lettering. No SVG interpreter,
video codec, or independent recreation of keyframes is needed at runtime. Use
`poster.png` as the static or reduced-motion fallback.

Center the artwork, preserve its aspect ratio, and fill the window with the
manifest's background color. Allow skipping by keyboard, mouse, or controller.
Show the static logo for reduced motion. Never delay a ready application for the
splash; Spice CAD should show it only while startup work is pending.

The frame manifest specifies 480 by 400 pixel cells, four columns and two rows
per page, and 73 frames sampled at 60 fps. Read cells left to right, then top to
bottom; switch pages every eight frames. Select
`min(floor(elapsed_seconds * fps), frame_count - 1)`. The final sampled frame is
at 1200 ms; hold it through the remaining duration, or until loading completes.
The pages have an opaque sRGB forest background; fill the surrounding window
with `background_rgba`. Preserve the 6:5 canvas. Show at most 480 by 400 logical
pixels to avoid enlarging the baked artwork. Do not loop.

Native players should decode off the UI thread, prefetch with a bounded queue,
reuse one atlas texture, and upload only on page changes. Release textures and
stop prefetching on dismissal. `HOLYFURRIES_REDUCED_MOTION=1` selects the static
poster in Spice CAD; other native applications should provide an equivalent
setting. The browser preview follows the system reduced-motion preference.

## Validate and export

Use `logos/avatar.png` for GitHub and other profile pictures. It is a 512 by 512
opaque PNG with a cream background and padding for circular crops.
`logos/avatar.svg` is its editable source, using the approved fox mark. Both
files are included in bundle exports; validation checks the PNG size.

To regenerate the avatar after editing its source:

```console
nix develop --command chromium --headless --no-sandbox --disable-gpu --hide-scrollbars --force-device-scale-factor=1 --window-size=512,512 --screenshot=logos/avatar.png "file://$PWD/logos/avatar.svg"
```

From this repository:

```console
nix run . -- validate
nix run . -- export output
nix flake check
```

The CLI checks manifests, SVG documents, animation durations, PNG canvas sizes,
and the source SHA-256 recorded in the frame manifest. Changing the SVG without
rebaking fails validation. Export creates a new directory containing the SVGs,
poster, atlas pages, manifests, logos, preview, documentation, and asset terms.
The destination must not exist. Pass a repository root as the final argument
when running from elsewhere.

To regenerate frames on Linux after editing the approved animation:

```console
nix develop --command cargo run --locked -- bake output
```

Review `output/`, then replace `splash/frames/` with that directory and run the
validation commands above. The development shell pins Chromium and supplies
Liberation fonts. `CHROMIUM` can override the executable outside that shell. The
baker samples paused browser animations at exact timestamps, flattens the
computed transforms and clipping, and paints the atlas pages. It refuses an
existing destination, bounds rendering to 120 seconds, and removes incomplete
output after failure. Browser dependencies are for asset generation only;
consumer applications embed the generated PNGs. Video export is not required.

Commit visual changes together with an updated PNG fallback. Review the
animation in Firefox and Chromium before publishing changes.

## Rights

Brand assets remain reserved to Holy Furries Studio; see `LICENSE`. The Rust
utility is MIT licensed separately under `tools/LICENSE`. The asset terms permit
building, running, and redistributing studio software and its forks, as well as
sharing screenshots. Unofficial builds must identify themselves as such. The
utility's MIT license grants no additional asset rights.
