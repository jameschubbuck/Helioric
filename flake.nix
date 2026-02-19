{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };
  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = ["rust-src"];
        };
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };
        helioric = rustPlatform.buildRustPackage {
          pname = "helioric";
          version = "1.0.0";
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          nativeBuildInputs = [pkgs.pkg-config pkgs.makeWrapper];
          postInstall = ''
            wrapProgram $out/bin/helioric \
              --prefix PATH : ${pkgs.lib.makeBinPath [pkgs.ddcutil pkgs.brightnessctl]}
          '';
        };
        helioric-static = pkgs.pkgsStatic.rustPlatform.buildRustPackage {
          pname = "helioric";
          version = "1.0.0";
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          nativeBuildInputs = [pkgs.pkg-config];
        };
      in {
        packages = {
          default = helioric;
          static = helioric-static;
        };
        apps = {
          default = flake-utils.lib.mkApp {
            drv = helioric;
          };
          dist = flake-utils.lib.mkApp {
            drv = pkgs.writeShellScriptBin "helioric-dist" ''
              mkdir -p dist
              echo "Building static binary..."
              cp ${helioric-static}/bin/helioric dist/helioric-linux-x86_64
              chmod +w dist/helioric-linux-x86_64
              strip dist/helioric-linux-x86_64
              echo "Built dist/helioric-linux-x86_64"
            '';
          };
        };
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = [
            rustToolchain
            pkgs.pkg-config
            pkgs.ddcutil
            pkgs.brightnessctl
          ];
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
        };
      }
    );
}
