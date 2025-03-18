{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";

    clippy-reviewdog-filter-src = {
      url = "github:qnighy/clippy-reviewdog-filter";
      flake = false;
    };
  };

  outputs =
    inputs@{ self, nixpkgs, ... }:
    let
      system = "x86_64-linux";

      pkgs = import nixpkgs {
        inherit system;
        overlays = [
          inputs.rust-overlay.overlays.default
        ];
      };

      rust-bin = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

      clippy-reviewdog-filter = pkgs.rustPlatform.buildRustPackage {
        pname = "clippy-reviewdog-filter";
        version = "${inputs.clippy-reviewdog-filter-src.shortRev}";

        src = inputs.clippy-reviewdog-filter-src;

        cargoHash = "sha256-pGkpgwCNXxKzoWuRIUSgXCJ2+PzrvD8voxtkDvWIazc=";
      };
    in
    {
      devShells.${system}.default = pkgs.mkShell {
        name = "ltrait";

        buildInputs = with pkgs; [
          rust-bin

          cargo-nextest

          # bench
          gnuplot
        ];
      };

      packages.${system} = rec {
        ci = pkgs.buildEnv {
          name = "ci";
          paths = with pkgs; [
            rust-bin

            cargo-nextest
            typos
          ];
        };
        review = pkgs.buildEnv {
          name = "ci";
          paths = with pkgs; [
            ci

            reviewdog

            clippy-reviewdog-filter
          ];
        };
      };
    };
}
