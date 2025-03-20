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
          (self: super: {
            # TODO: waiting for https://github.com/NixOS/nixpkgs/pull/391472
            # HACK:
            cargo-codspeed = super.rustPlatform.buildRustPackage rec {
              pname = "cargo-codspeed";
              version = "2.9.1";

              src = super.fetchFromGitHub {
                owner = "CodSpeedHQ";
                repo = "codspeed-rust";
                rev = "v${version}";
                hash = "sha256-q5xsZ8KHuC/Qm+o4xcWbW9Y9VrxHZ+/AxUO8TYEbE74=";
              };

              useFetchCargoVendor = true;
              cargoHash = "sha256-Ance7Hfl0EOmMfZf3ZqvawrK7scot7WpefLtemHKb+U=";

              nativeBuildInputs = with super; [
                curl
                pkg-config
              ];

              buildInputs =
                with super;
                [
                  curl
                  libgit2
                  openssl
                  zlib
                ]
                ++ lib.optionals stdenv.hostPlatform.isDarwin [
                  darwin.apple_sdk.frameworks.Security
                ];

              cargoBuildFlags = [ "-p=cargo-codspeed" ];
              cargoTestFlags = cargoBuildFlags;
              checkFlags = [
                # requires an extra dependency, blit
                "--skip=test_package_in_deps_build"
              ];

              env = {
                LIBGIT2_NO_VENDOR = 1;
              };
            };
          })
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

      packages.${system} = {
        ci = pkgs.buildEnv {
          name = "ci";
          paths = with pkgs; [
            rust-bin

            cargo-nextest
            typos

            # bench
            gnuplot
            cargo-codspeed
          ];
        };
        review = pkgs.buildEnv {
          name = "review";
          paths = with pkgs; [
            rust-bin

            cargo-nextest
            typos

            reviewdog

            clippy-reviewdog-filter
          ];
        };
      };
    };
}
