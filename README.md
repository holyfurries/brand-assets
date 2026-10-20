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
- `logos/`: fox-and-halo mark and favicon.
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
preview demonstrates replay and skip handling. Native applications should use
the PNG as a static fallback or reproduce the SVG keyframes in their animation
system. Static SVG renderers may ignore animation or lack OKLCH support.

Center the artwork, preserve its aspect ratio, and fill the window with the
manifest's background color. Allow skipping by keyboard, mouse, or controller.
Show the static logo for reduced motion. Never delay a ready application for the
splash; Spice CAD should show it only while startup work is pending.

## Validate and export

From this repository:

```console
nix run . -- validate
nix run . -- export output
nix flake check
```

The CLI checks the manifest, SVG documents, animation durations, and PNG canvas
size. Export creates a new directory containing the approved SVGs, PNG, logos,
manifest, preview, documentation, and asset terms. It copies the reviewed PNG;
it does not rasterize SVGs or produce a video. The destination must not exist.
Pass a repository root as the final argument when running from elsewhere.

Commit visual changes together with an updated PNG fallback. Review the
animation in Firefox and Chromium before publishing changes.

## Rights

Brand assets remain reserved to Holy Furries Studio; see `LICENSE`. The Rust
utility is MIT licensed separately under `tools/LICENSE`.
