{
  description = "takt – Rust CLI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {nixpkgs, fenix, ...}: let
    forAllSystems = nixpkgs.lib.genAttrs ["x86_64-linux" "aarch64-linux"];
  in {
    devShells = forAllSystems (system: let
      pkgs = nixpkgs.legacyPackages.${system};
      toolchain = fenix.packages.${system}.stable.withComponents [
        "cargo" "clippy" "rust-src" "rustc" "rustfmt"
      ];
    in {
      default = pkgs.mkShell {
        nativeBuildInputs = [
          toolchain
          fenix.packages.${system}.rust-analyzer
          pkgs.pkg-config
          pkgs.cargo-expand
        ];
      };
    });
  };
}
