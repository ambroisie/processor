{
  description = "A simple payment processor";

  inputs = {
    flake-utils = {
      type = "github";
      owner = "numtide";
      repo = "flake-utils";
      ref = "master";
    };

    naersk = {
      type = "github";
      owner = "nix-community";
      repo = "naersk";
      ref = "master";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };

    nixpkgs = {
      type = "github";
      owner = "NixOS";
      repo = "nixpkgs";
      ref = "nixpkgs-unstable";
    };

    pre-commit-hooks = {
      type = "github";
      owner = "cachix";
      repo = "pre-commit-hooks.nix";
      ref = "master";
      inputs = {
        flake-utils.follows = "flake-utils";
        nixpkgs.follows = "nixpkgs";
      };
    };

    rust-overlay = {
      type = "github";
      owner = "oxalica";
      repo = "rust-overlay";
      ref = "master";
      inputs = {
        flake-utils.follows = "flake-utils";
        nixpkgs.follows = "nixpkgs";
      };
    };
  };

  outputs =
    { self
    , flake-utils
    , naersk
    , nixpkgs
    , pre-commit-hooks
    , rust-overlay
    }:
    let
      inherit (flake-utils.lib) eachSystem system;

      mySystems = [
        system.aarch64-linux
        system.x86_64-darwin
        system.x86_64-linux
      ];

      eachMySystem = eachSystem mySystems;
    in
    eachMySystem (system:
    let
      overlays = [ (import rust-overlay) ];
      pkgs = import nixpkgs { inherit overlays system; };
      my-rust = pkgs.rust-bin.stable.latest.default.override {
        extensions = [ "rust-src" ];
      };
      naersk-lib = naersk.lib."${system}".override {
        cargo = my-rust;
        rustc = my-rust;
      };
      inherit (pkgs) lib;
      pre-commit =
        let
          # See https://github.com/cachix/pre-commit-hooks.nix/issues/126
          rust-env = pkgs.buildEnv {
            name = "rust-env";
            buildInputs = [ pkgs.makeWrapper ];
            paths = [ my-rust ];
            pathsToLink = [ "/" "/bin" ];
            postBuild = ''
              for i in $out/bin/*; do
                wrapProgram "$i" --prefix PATH : "$out/bin"
              done
            '';
          };
        in
        pre-commit-hooks.lib.${system}.run {
          src = self;

          hooks = {
            clippy = {
              enable = true;
              entry = lib.mkForce "${rust-env}/bin/cargo-clippy clippy";
            };

            nixpkgs-fmt = {
              enable = true;
            };

            rustfmt = {
              enable = true;
              entry = lib.mkForce "${rust-env}/bin/cargo-fmt fmt -- --check --color always";
            };
          };
        };
    in
    rec {

      devShells = {
        default = pkgs.mkShell {
          inputsFrom = [
            packages.processor
          ];

          nativeBuildInputs = with pkgs; [
            rust-analyzer
            # Clippy, rustfmt, etc...
            my-rust
          ];

          inherit (pre-commit) shellHook;

          RUST_SRC_PATH = "${my-rust}/lib/rustlib/src/rust/library";
        };
      };

      packages = {
        default = self.packages."${system}".processor;

        processor = naersk-lib.buildPackage {
          src = self;

          doCheck = true;

          passthru = {
            inherit my-rust;
          };
        };
      };
    });
}
