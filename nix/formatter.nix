# nix/formatter.nix
{
  pkgs,
  inputs,
  system,
  ...
}: let
  biomeCmd = "${pkgs.biome}/bin/biome";
  shellhardenCmd = "${pkgs.shellharden}/bin/shellharden";
  fishIndentCmd = "${pkgs.fish}/bin/fish_indent";

  tf = inputs.treefmt-nix.lib.evalModule pkgs {
    projectRootFile = "flake.nix";

    programs = {
      alejandra.enable = true; # *.nix
      rustfmt.enable = true; # Rust
      taplo.enable = true; # TOML
      stylua.enable = true; # Lua
    };

    settings = {
      global.excludes = ["./result/*" "./target/*"];

      formatter = {
        biome = {
          command = biomeCmd;
          options = ["format" "--write"];
          includes = ["*.js" "*.cjs" "*.mjs" "*.jsx" "*.ts" "*.tsx" "*.html" "*.css"];
        };

        shellharden = {
          command = shellhardenCmd;
          options = ["--transform" "--replace"];
          includes = ["*.sh"];
        };

        fish = {
          command = fishIndentCmd;
          options = ["-w"];
          includes = ["*.fish"];
        };
      };
    };
  };
in
  tf.config.build
