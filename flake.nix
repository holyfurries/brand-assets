{
  description = "Holy Furries Studio brand assets";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [ inputs.devshell.flakeModule ];
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];
      perSystem =
        { pkgs, ... }:
        let
          rust_toolchain = (inputs.rust-overlay.lib.mkRustBin { } pkgs).stable.latest.default.override {
            extensions = [
              "rustfmt"
              "clippy"
              "rust-src"
            ];
          };
          crane_lib = (inputs.crane.mkLib pkgs).overrideToolchain rust_toolchain;
          common_args = {
            pname = "brand-assets";
            version = (builtins.fromTOML (builtins.readFile ./tools/Cargo.toml)).package.version;
            src = pkgs.lib.cleanSource ./.;
            strictDeps = true;
          };
          cargo_artifacts = crane_lib.buildDepsOnly common_args;
          package = crane_lib.buildPackage (
            common_args
            // {
              cargoArtifacts = cargo_artifacts;
              meta.mainProgram = "brand-assets";
            }
          );
          bundle = pkgs.runCommand "holyfurries-brand-assets" { nativeBuildInputs = [ package ]; } ''
            brand-assets validate ${inputs.self}
            brand-assets export "$out" ${inputs.self}
            brand-assets validate "$out"
          '';
        in
        {
          packages = {
            default = package;
            inherit bundle;
          };
          checks = {
            assets = bundle;
            tools = package;
            format = crane_lib.cargoFmt common_args;
            clippy = crane_lib.cargoClippy (
              common_args
              // {
                cargoArtifacts = cargo_artifacts;
                cargoClippyExtraArgs = "--all-targets -- --deny warnings";
              }
            );
          };
          devshells.default.packages = with pkgs; [
            rust_toolchain
            stdenv.cc
            miniserve
            ripgrep
            nixfmt
            libxml2
            superhtml
            biome
            mdformat
            python3Packages.mdformat-gfm
          ];
          formatter = pkgs.nixfmt;
        };
    };
}
