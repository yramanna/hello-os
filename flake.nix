{
  description = "Hello OS";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, ... }: let
    # System types to support.
    supportedSystems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];

    # Rust nightly version.
    nightlyVersion = "2025-08-20";
  in flake-utils.lib.eachSystem supportedSystems (system: let
    makeNixpkgs = system: import nixpkgs {
      inherit system;
      overlays = [
        rust-overlay.overlays.default
      ];
    };

    pkgs = makeNixpkgs system;
    x86Pkgs = makeNixpkgs "x86_64-linux";
    x86Tools = pkgs.pkgsCross.gnu64;

    rustNightly = pkgs.rust-bin.nightly.${nightlyVersion}.default.override {
      extensions = [ "rust-src" "rust-analyzer-preview" ];
      targets = [ "x86_64-unknown-linux-gnu" ];
    };
  in {
    devShell = pkgs.mkShell {
      nativeBuildInputs = with pkgs; [
        rustNightly

        qemu
        gdb

        # Toolchain
        nasm
        (pkgs.writeShellScriptBin "x86_64.ld" ''
          exec ${x86Tools.buildPackages.bintools}/bin/${x86Tools.stdenv.cc.targetPrefix}ld "$@"
        '')

        x86Pkgs.grub2
        xorriso
      ];
    };
  });
}
