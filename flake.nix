{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      system = "x86_64-linux";
      overlays = [ (import rust-overlay) ];
      pkgs = import nixpkgs {
        inherit system overlays;
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
          dioxus-cli
          python314
          python314Packages.uv
          ungoogled-chromium
          wasm-pack
        ];
        shellHook = ''
          uv sync
          source .venv/bin/activate
        '';
      };
    };
}
