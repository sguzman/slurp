# nix/packages/holodeck/default.nix
{
  pkgs,
  inputs,
  ...
}: let
  cargoToml = builtins.fromTOML (builtins.readFile ../../../Cargo.toml);
  name = cargoToml.package.name;
  lib = pkgs.lib;
  naersk = inputs.naersk;
  mkPkgConfigPath = pkgsList:
    lib.makeSearchPath "lib/pkgconfig" (map lib.getDev pkgsList);
in
  naersk.lib.${pkgs.system}.buildPackage {
    pname = name;
    version = cargoToml.package.version;

    src = ../../..;

    cargoToml = ../../../Cargo.toml;
    cargoLock = ../../../Cargo.lock;

    nativeBuildInputs = with pkgs; [
      clang
      llvmPackages.libclang
      pkg-config
    ];

    buildInputs = with pkgs; [
      openssl
    ];

    PKG_CONFIG_PATH = mkPkgConfigPath (with pkgs; [
      openssl
    ]);

    OPENSSL_NO_VENDOR = "1";
  }
