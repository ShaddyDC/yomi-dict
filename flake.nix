{
  description = "Yomi-Dict is a yomidict dictionary reader";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";

    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = { url = "github:oxalica/rust-overlay"; };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
          };
        in
        with pkgs;
        {
          devShells.default = stdenv.mkDerivation {
            name = "rust-env";
            # Build-time Additional Dependencies
            nativeBuildInputs = [
              cargo
              rustfmt
              rust-analyzer
              clippy
              wasm-pack
              pkgconfig
              openssl

              (rust-bin.stable.latest.default.override {
                extensions = [ "rust-src" ];
                targets = [ "wasm32-unknown-unknown" ];
              })
            ];
            buildInputs = [
              geckodriver
              chromedriver
            ];

            # Set Environment Variables
            RUST_BACKTRACE = 1;
          };
        }
      );
}
