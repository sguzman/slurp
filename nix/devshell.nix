# nix/devshell.nix
{
  pkgs,
  perSystem,
  ...
}: let
  cargoToml = builtins.fromTOML (builtins.readFile ../Cargo.toml);
  name = cargoToml.package.name;
  lib = pkgs.lib;
  toolPkgs = with pkgs; [
    # Rust toolchain
    rustc
    cargo
    rustfmt
    clippy

    # Release + changelog
    cargo-release
    git-cliff

    # Linker/tooling for fast builds
    mold
    clang
    llvmPackages.bintools

    # Common native deps many crates need
    pkg-config
    openssl

    # Handy utilities
    jq
    curl

    # Formatting/linting for Nix
    alejandra
    statix
    deadnix
    taplo
    stylua
    fish

    # SurrealDB CLI (optional, nice for quick checks)
    surrealdb
  ];
  mkPkgConfigPath = pkgsList:
    lib.makeSearchPath "lib/pkgconfig" (map lib.getDev pkgsList);

  fmtPkg = p: let
    pn =
      if p ? pname
      then p.pname
      else (p.name or "pkg");
    ver =
      if p ? version
      then p.version
      else "";
    lbl =
      if ver == ""
      then "${pn}"
      else "${pn} ${ver}";
  in ''echo "        • ${lbl}"'';

  packagesSummaryFish = builtins.concatStringsSep "\n" (map fmtPkg toolPkgs);

  banner = pkgs.writeText "${name}-banner.fish" ''
        function fish_greeting
          set_color -o cyan
          echo "${name} devshell"
          set_color normal

          # Where & who
          set_color brwhite; echo; echo "Project:"; set_color normal
          echo "  • PWD     → "(pwd)
          if command -q git
            if git rev-parse --is-inside-work-tree 2>/dev/null
              echo -n "  • branch  → "
              git rev-parse --abbrev-ref HEAD 2>/dev/null
            end
          end

          # Packages (auto-generated from Nix)
          set_color brwhite; echo; echo "Packages (from Nix):"; set_color normal
    ${packagesSummaryFish}

          # Dynamic commands menu
          set_color brwhite; echo; echo "Menu (devshell commands):"; set_color normal
          if type -q menu
            menu
          else
            echo "  (menu unavailable)"
          end

          echo
          set_color brwhite; echo "Tip:"; set_color normal
          echo "  • Run 'devhelp' anytime to reprint this banner."
          echo
        end

        # Reprint on demand
        function devhelp
          fish_greeting
        end
  '';
in
  perSystem.devshell.mkShell {
    packages = toolPkgs;

    # Source the banner file, then start fish
    devshell.interactive.fish.text = "exec ${pkgs.fish}/bin/fish -C 'source ${banner}'";

    motd = "";

    env = [
      {
        name = "SHELL";
        value = "${pkgs.fish}/bin/fish";
      }
      {
        name = "OPENSSL_DIR";
        value = "${pkgs.openssl.dev}";
      }
      {
        name = "OPENSSL_LIB_DIR";
        value = "${pkgs.openssl.out}/lib";
      }
      {
        name = "OPENSSL_INCLUDE_DIR";
        value = "${pkgs.openssl.dev}/include";
      }
    ];

    commands = [
      {
        name = "devhelp";
        help = "reprint this banner/help";
        command = "${pkgs.fish}/bin/fish -c devhelp";
      }

      {
        name = "build";
        help = "nix build .#${name}";
        command = "nix build .#${name}";
      }
      {
        name = "run";
        help = "cargo run --";
        command = "cargo run --";
      }

      {
        name = "fmt";
        help = "format Nix + Rust (treefmt: alejandra+rustfmt)";
        command = "nix fmt";
      }
      {
        name = "fmt:nix";
        help = "format Nix only (Alejandra)";
        command = "alejandra .";
      }
      {
        name = "fmt:rust";
        help = "format Rust only (cargo fmt)";
        command = "cargo fmt --all";
      }
      # TOML
      {
        name = "fmt:toml";
        help = "format TOML (taplo)";
        command = "taplo fmt .";
      }
      # Lua
      {
        name = "fmt:lua";
        help = "format Lua (stylua)";
        command = "stylua .";
      }
      # JS/TS/HTML/CSS
      {
        name = "fmt:web";
        help = "format JS/TS/HTML/CSS (biome)";
        command = "${pkgs.biome}/bin/biome format --write .";
      }
      # Shell
      {
        name = "fmt:sh";
        help = "format shell (shellharden)";
        command = "${pkgs.shellharden}/bin/shellharden -i (fd -e sh -e bash)";
      }
      # Fish
      {
        name = "fmt:fish";
        help = "format fish scripts";
        command = "fd -e fish -X ${pkgs.fish}/bin/fish_indent --write";
      }

      {
        name = "check";
        help = "cargo check (all targets)";
        command = "cargo check --all-targets";
      }
      {
        name = "test";
        help = "cargo test";
        command = "cargo test";
      }

      {
        name = "lint:nix";
        help = "Nix lint: statix + deadnix";
        command = "statix check . && deadnix .";
      }
      {
        name = "fix:nix";
        help = "Auto-fix with statix";
        command = "statix fix .";
      }
    ];
  }
