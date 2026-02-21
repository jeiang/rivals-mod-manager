{
  description = "Tauri v2 devshell with latest Rust nightly";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        rustNightly = pkgs.rust-bin.nightly.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        isLinux = pkgs.stdenv.isLinux;
        isDarwin = pkgs.stdenv.isDarwin;

        common = with pkgs; [
          rustNightly
          pkg-config
          openssl
          cacert
          bun
        ];

        # Tauri v2 Linux deps: webkit2gtk 4.1 + gtk3 (+ tray libs if used)
        tauriLinux = with pkgs; [
          gtk3
          webkitgtk_4_1
          librsvg
          glib
          libsoup_3
          glib-networking

          # Only needed if you use the system tray:
          libayatana-appindicator
        ];

        tauriDarwin = with pkgs; [
          darwin.apple_sdk.frameworks.AppKit
          darwin.apple_sdk.frameworks.WebKit
          darwin.apple_sdk.frameworks.CoreServices
          darwin.apple_sdk.frameworks.Security
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          packages =
            common
            ++ pkgs.lib.optionals isLinux tauriLinux
            ++ pkgs.lib.optionals isDarwin tauriDarwin;

          OPENSSL_NO_VENDOR = 1;

          shellHook = ''
            export RUST_SRC_PATH="${rustNightly}/lib/rustlib/src/rust/library"
            echo "rustc: $(rustc --version)"
            echo "node:  $(bun --version)"
          '';
        };
      });
}
