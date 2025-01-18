# nixpkgs-24.11-darwin 2025-1-1 from https://status.nixos.org/ for rustc > 1.77.2 (I think 1.82)
# This is derived from zed's nix/shell.nix
{
  pkgs ? import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/464fe85c27bd5761781a2773526ef9c1b0184dda.tar.gz")
  { overlays = [(import (builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/fc839c9d5d1ebc789b4657c43c4d54838c7c01de.tar.gz"))
                (final: prev: {
                   rustToolchain = final.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
                })]; },
}:
let
  inherit (pkgs) lib;
in
pkgs.mkShell rec {
  packages = [
    pkgs.clang
    pkgs.curl
    pkgs.cmake
    pkgs.perl
    pkgs.pkg-config
    pkgs.protobuf
    pkgs.rustPlatform.bindgenHook
    pkgs.rust-analyzer
    # nathan added:
    pkgs.xcbuild
  ];

  buildInputs =
    [
      pkgs.curl
      pkgs.fontconfig
      pkgs.freetype
      pkgs.libgit2
      pkgs.openssl
      pkgs.sqlite
      pkgs.zlib
      pkgs.zstd
      pkgs.rustToolchain
    ]
    ++ lib.optionals pkgs.stdenv.hostPlatform.isLinux [
      pkgs.alsa-lib
      pkgs.libxkbcommon
    ]
    ++ lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
      pkgs.apple-sdk_15
    ];

  # We set SDKROOT and DEVELOPER_DIR to the Xcode ones instead of the nixpkgs ones,
  # because we need Swift 6.0 and nixpkgs doesn't have it.`
  # Xcode is required for development anyways
  # (nds: I set it back to nixpkgs ones, I don't need Swift)
  shellHook =
    ''
      export LD_LIBRARY_PATH="${lib.makeLibraryPath buildInputs}:$LD_LIBRARY_PATH"
      export PROTOC="${pkgs.protobuf}/bin/protoc"
    '' + lib.optionalString pkgs.stdenv.hostPlatform.isDarwin ''
       export SDKROOT=${pkgs.apple-sdk_15}/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk;
       export DEVELOPER_DIR=${pkgs.apple-sdk_15};
     ''; # Sounds like this should be done for me? Why do I have to do it?
    # + lib.optionalString pkgs.stdenv.hostPlatform.isDarwin ''
    #   export SDKROOT="/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk";
    #   export DEVELOPER_DIR="/Applications/Xcode.app/Contents/Developer";
    # '';

  FONTCONFIG_FILE = pkgs.makeFontsConf {
    fontDirectories = [
      "./assets/fonts/zed-mono"
      "./assets/fonts/zed-sans"
    ];
  };
  ZSTD_SYS_USE_PKG_CONFIG = true;
}
