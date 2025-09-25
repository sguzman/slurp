# nix/packages/holodeck/default.nix
{
  pkgs,
  inputs,
  ...
}: let
  cargoToml = builtins.fromTOML (builtins.readFile ../../../Cargo.toml);
  name = cargoToml.package.name;
  naersk = inputs.naersk;
in
  naersk.lib.${pkgs.system}.buildPackage {
    pname = name;
    version = cargoToml.package.version;

    src = ../../../..;

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
  }
