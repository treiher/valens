{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    nixpkgs-unstable.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, nixpkgs-unstable, rust-overlay }:
    let
      system = "x86_64-linux";
      overlays = [ (import rust-overlay) ];
      pkgs = import nixpkgs {
        inherit system overlays;
      };
      unstable = import nixpkgs-unstable {
        inherit system;
      };
      rustfmtDate = nixpkgs.lib.removePrefix "nightly-"
        (nixpkgs.lib.importTOML ./rustfmt-toolchain.toml).toolchain.channel;
      nightlyRustfmt = pkgs.rust-bin.nightly.${rustfmtDate}.rustfmt;
      fontsConf = pkgs.replaceVars ./tools/fonts.conf {
        fonts = "${pkgs.dejavu_fonts.minimal}/share/fonts";
      };
    in {
      devShells.${system}.default = with pkgs; mkShell {
        packages = [
          (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
          binaryen
          cargo-llvm-cov
          cargo-nextest
          chromedriver
          dart-sass
          dejavu_fonts.minimal
          # `dx` must match the `dioxus` version in `Cargo.lock`, which the stable channel predates.
          unstable.dioxus-cli
          imagemagick
          playwright-driver.browsers
          python314
          python314Packages.uv
          ungoogled-chromium
          wasm-pack
        ];
        env = {
          FONTCONFIG_FILE = "${fontsConf}";
          LD_LIBRARY_PATH = lib.makeLibraryPath [ stdenv.cc.cc ];
          PLAYWRIGHT_BROWSERS_PATH = "${playwright-driver.browsers}";
          PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS = "true";
          RUSTFMT = "${nightlyRustfmt}/bin/rustfmt";
        };
        shellHook = ''
          uv sync
          source .venv/bin/activate
        '';
      };
    };
}
