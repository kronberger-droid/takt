{
  description = "takt – Rust CLI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = {
    self,
    nixpkgs,
    ...
  }: let
    forAllSystems = nixpkgs.lib.genAttrs [
      "x86_64-linux"
      "aarch64-linux"
      "aarch64-darwin"
    ];
  in {
    packages = forAllSystems (system: let
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      default = pkgs.rustPlatform.buildRustPackage {
        pname = "takt";
        version = "0.2.0";
        src = self;
        cargoLock.lockFile = ./Cargo.lock;
        meta = {
          description = "Time tracking CLI with hierarchical tags and human-readable storage.";
          homepage = "https://github.com/kronberger-droid/takt";
          license = pkgs.lib.licenses.mit;
          mainProgram = "takt";
        };
      };
    });

    devShells = forAllSystems (system: let
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      default = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          cargo
          clippy
          rustc
          rustfmt
          rust-analyzer
          pkg-config
          cargo-expand
        ];
        RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
      };
    });
  };
}
