{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    nixpkgs,
    flake-utils,
    rust-overlay,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [rust-overlay.overlays.default];
        };
        custom-rust-bin = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        rustPlatform = pkgs.makeRustPlatform {
          cargo = custom-rust-bin;
          rustc = custom-rust-bin;
        };

        build = pname: src:
          rustPlatform.buildRustPackage {
            inherit pname src;
            version = (builtins.fromTOML (builtins.readFile "${src}/Cargo.toml")).package.version;

            nativeBuildInputs = with pkgs; [
              pkg-config
              zlib
            ];

            buildInputs = with pkgs; [
              openssl
            ];
            cargoLock = {
              lockFile = "${src}/Cargo.lock";
            };

            meta = {
              mainProgram = pname;
            };
          };
      in {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            pkg-config
            zlib
            openssl
          ];
          packages = [
            custom-rust-bin
          ];
        };

        packages.default = build "git-patcher" ./.;
      }
    )
    // {
      lib = {
        applyPatches = {
          src,
          upstream,
          pkgs,
          ...
        }:
          pkgs.runCommand "apply-patches" {
            buildInputs = [pkgs.git];
          } ''
            mkdir -p $out
            cp -r ${upstream}/. $out/
            cd $out
            git init
            while IFS= read -r line; do
                echo "Applying patch: $line"
                git apply "$src/patches/$line"
            done < "$src/patches/series"
            echo "All patches applied successfully."
            rm -rf .git
          '';
      };
    };
}
