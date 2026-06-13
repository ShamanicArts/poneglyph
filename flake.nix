{
  description = "A terminal album art viewer for mpd, now made in Rust!";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    git-hooks.url = "github:cachix/git-hooks.nix";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      ...
    }:
    let
      inherit (self) inputs;
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      overlays = [ (import rust-overlay) ];
      forAllSystems =
        f:
        nixpkgs.lib.genAttrs systems (
          system:
          f {
            system = system;
            pkgs = import nixpkgs { inherit system overlays; };
          }
        );
      mkPoneglyph =
        package: pkgs:
        pkgs.callPackage ./nix/${package}.nix {
          rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        };
    in
    {
      formatter = forAllSystems (
        { pkgs, system }:
        let
          config = self.checks.${system}.pre-commit-check.config;
          inherit (config) package configFile;
          script = ''
            ${pkgs.lib.getExe package} run --all-files --config ${configFile}
          '';
        in
        pkgs.writeShellScriptBin "pre-commit-run" script
      );

      packages = forAllSystems (
        { pkgs, system }:
        {
          default = mkPoneglyph "build" pkgs;
          dev = mkPoneglyph "dev" pkgs;
          dev-shell = mkPoneglyph "dev-shell" pkgs;
          poneglyph = self.packages.${system}.default;
        }
      );

      overlays = {
        default = final: _: {
          poneglyph = mkPoneglyph "build" final;
        };
        poneglyph = self.overlays.default;
      };

      checks = forAllSystems (
        { pkgs, system }:
        {
          pre-commit-check = inputs.git-hooks.lib.${system}.run {
            src = ./.;
            settings.rust = {
              check.cargoDeps = pkgs.rustPlatform.importCargoLock { lockFile = ./Cargo.lock; };
            };
            hooks = {
              nixfmt.enable = true;

              cargo-fmt =
                let
                  rust-toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
                  clippy = pkgs.writeShellApplication {
                    name = "cargo-fmt";
                    runtimeInputs = [
                      rust-toolchain
                    ];
                    text = ''
                      export LIBCLANG_PATH="${pkgs.libclang.lib}/lib"
                      ${rust-toolchain}/bin/cargo fmt --all -- --check
                    '';
                  };
                in
                {
                  enable = true;
                  extraPackages = with pkgs; [
                    pkg-config
                    clang
                    rust-toolchain
                  ];
                  package = clippy;
                  entry = "${pkgs.lib.getExe clippy}";
                  pass_filenames = false;
                };

              cargo-clippy =
                let
                  rust-toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
                  clippy = pkgs.writeShellApplication {
                    name = "cargo-clippy";
                    runtimeInputs = [
                      rust-toolchain
                    ];
                    text = ''
                      export LIBCLANG_PATH="${pkgs.libclang.lib}/lib"
                      ${rust-toolchain}/bin/cargo clippy --all-targets -- -D warnings
                    '';
                  };
                in
                {
                  enable = true;
                  extraPackages = with pkgs; [
                    pkg-config
                    clang
                    rust-toolchain
                  ];
                  package = clippy;
                  entry = "${pkgs.lib.getExe clippy}";
                  pass_filenames = false;
                };

              cargo-test =
                let
                  rust-toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
                  test = pkgs.writeShellApplication {
                    name = "cargo-test";
                    runtimeInputs = [
                      rust-toolchain
                    ];
                    text = ''
                      export LIBCLANG_PATH="${pkgs.libclang.lib}/lib"
                      ${rust-toolchain}/bin/cargo test --locked
                    '';
                  };
                in
                {
                  enable = true;
                  extraPackages = with pkgs; [
                    pkg-config
                    clang
                    rust-toolchain
                  ];
                  package = test;
                  entry = "${pkgs.lib.getExe test}";
                  pass_filenames = false;
                };
            };
            package = pkgs.prek;
          };
        }
      );

      devShells = forAllSystems (
        { pkgs, system }:
        let
          inherit (self.checks.${system}.pre-commit-check) shellHook enabledPackages;
        in
        {
          default = pkgs.mkShell {
            buildInputs = enabledPackages;

            nativeBuildInputs = [
              self.packages.${system}.dev-shell
            ];

            shellHook = shellHook + ''
              cargo build
            '';

            LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
          };
        }
      );

      homeModules.default = import ./nix/hm.nix self;
    };
}
