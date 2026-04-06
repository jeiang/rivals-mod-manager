{
  description = "Build a cargo project";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";

    flake-utils.url = "github:numtide/flake-utils";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      flake-utils,
      advisory-db,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        inherit (pkgs) lib;

        craneLib = crane.mkLib pkgs;
        src = let
          eguiAssets = path: _type: builtins.match ".*\.png$" path != null;
          filters = path: type:
            (eguiAssets path type) || (craneLib.filterCargoSources path type);
        in lib.cleanSourceWith {
          src = ./.;
          filter = filters;
          name = "source"; # Be reproducible, regardless of the directory name
        };

        commonArgs = {
          inherit src;
          strictDeps = true;

          buildInputs = with pkgs; [
            trunk

            # misc. libraries
            openssl
            pkg-config

            # GUI libs
            libxkbcommon
            libGL
            fontconfig

            # wayland libraries
            wayland

            # x11 libraries
            libxcursor
            libxrandr
            libxi
            libx11
          ];
          LD_LIBRARY_PATH = "${lib.makeLibraryPath commonArgs.buildInputs}";
        };

        # Build *just* the cargo dependencies, so we can reuse
        # all of that work (e.g. via cachix) when running in CI
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the actual crate itself, reusing the dependency
        # artifacts from above.
        rivals-mod-manager = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            doCheck = false;
          }
        );
      in
      {
        checks = {
          inherit rivals-mod-manager;

          rivals-mod-manager-clippy = craneLib.cargoClippy (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--all-targets -- --deny warnings";
            }
          );

          # rivals-mod-manager-doc = craneLib.cargoDoc (
          #   commonArgs
          #   // {
          #     inherit cargoArtifacts;
          #     env.RUSTDOCFLAGS = "--deny warnings";
          #   }
          # );

          # Check formatting
          rivals-mod-manager-fmt = craneLib.cargoFmt {
            inherit src;
          };

          rivals-mod-manager-toml-fmt = craneLib.taploFmt {
            src = pkgs.lib.sources.sourceFilesBySuffices src [ ".toml" ];
          };

          # Audit dependencies
          rivals-mod-manager-audit = craneLib.cargoAudit {
            inherit src advisory-db;
          };

          rivals-mod-manager-nextest = craneLib.cargoNextest (
            commonArgs
            // {
              inherit cargoArtifacts;
              partitions = 1;
              partitionType = "count";
              cargoNextestPartitionsExtraArgs = "--no-tests=pass";
            }
          );
        };

        packages = {
          default = rivals-mod-manager;
        };

        apps.default = flake-utils.lib.mkApp {
          drv = rivals-mod-manager;
        };

        devShells.default = craneLib.devShell {
          inherit (commonArgs) LD_LIBRARY_PATH;
          checks = self.checks.${system};
          packages = with pkgs; [
            rust-analyzer
            rustPackages.clippy
            tokei
          ];
        };
      }
    );
}
