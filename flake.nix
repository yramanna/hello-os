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
    x86CrossPkgs = if pkgs.system == "x86_64-linux" then pkgs else pkgs.pkgsCross.gnu64;

    inherit (pkgs) lib;

    rustNightly = pkgs.rust-bin.nightly.${nightlyVersion}.default.override {
      extensions = [ "rust-src" "rust-analyzer-preview" ];
      targets = [ "x86_64-unknown-linux-gnu" ];
    };

    rustNightlyMinimal = pkgs.rust-bin.nightly.${nightlyVersion}.minimal.override {
      extensions = [ "rust-src" ];
      targets = [ "x86_64-unknown-linux-gnu" ];
    };

    commonTools = with pkgs; [
      # Toolchain
      nasm
      (pkgs.writeShellScriptBin "x86_64.ld" ''
        exec ${x86CrossPkgs.buildPackages.bintools}/bin/${x86CrossPkgs.stdenv.cc.targetPrefix}ld "$@"
      '')
    ];
  in {
    devShell = x86CrossPkgs.mkShell {
      nativeBuildInputs = with pkgs; ([
        rustNightly
        qemu
        gdb
      ] ++ commonTools ++ lib.optionals pkgs.stdenv.isLinux [
        grub2
        xorriso
      ]);

      GRUB_X86_MODULES = lib.optionalString pkgs.stdenv.isLinux "${x86Pkgs.grub2}/lib/grub/i386-pc";
    };
    devShells.autograder = x86CrossPkgs.mkShell {
      nativeBuildInputs = with pkgs; [
        rustNightlyMinimal
        qemu_test
      ] ++ commonTools;

      # Ugly hack to prebuild a shell
      buildPhase = ''
        mkdir -p $out
        (
          # nix/src/nix-build/nix-build.cc
          unset TZ
          unset NIX_ENFORCE_PURITY
          export dontAddDisableDepTrack=1

          # Variables whose values we cannot dictate here
          unset SHELL
          unset TERM
          unset SHLVL
          unset PWD
          unset OLDPWD

          unset HOME

          unset TMP
          unset TEMP
          unset TMPDIR
          unset TEMPDIR

          unset NIX_BUILD_TOP
          unset NIX_BUILD_CORES

          unset NIX_LOG_FD

          echo 'OLDPATH="$PATH"' >>$out/profile
          declare -x >>$out/profile
          echo 'export PATH="$PATH:$OLDPATH"' >>$out/profile
        )
      '';
    };
  });
}
