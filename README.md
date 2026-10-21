# Holy Furries brand assets

Shared studio logos and startup splash.

- `logos/`: studio mark, favicon, and profile avatar (`avatar.png`).
- `splash/source.svg`: animated splash for web applications.
- `splash/frames/`: baked PNG atlases and playback manifest for native
  applications.
- `splash/poster.png`: static fallback.
- `tools/`: Rust validation and export utility.

The splash lasts 1.5 seconds. Preserve its aspect ratio, allow skipping, and use
the static fallback for reduced motion. Playback details are in
`splash/manifest.json` and `splash/frames/manifest.json`.

## Preview

```console
nix develop
miniserve . --index preview.html --port 8081 --interfaces 127.0.0.1
```

Open <http://localhost:8081/splash/>.

## Validate and export

```console
nix run . -- validate
nix run . -- export output
nix flake check
```

After editing the animation, regenerate the atlases with
`nix develop --command cargo run --locked -- bake output`, replace
`splash/frames/` with the result, and update the static fallback. Export and
bake require a destination that does not already exist.

## License

Brand assets are reserved to Holy Furries Studio; see [LICENSE](LICENSE). The
Rust utility is [MIT licensed](tools/LICENSE).
