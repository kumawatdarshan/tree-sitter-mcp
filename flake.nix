{
  description = "Tree Sitter MCP Server";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    crane.url = "github:ipetkov/crane";
  };

  outputs = {
    self,
    nixpkgs,
    fenix,
    flake-utils,
    crane,
    treefmt-nix,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [fenix.overlays.default];
      };

      craneLib = crane.mkLib pkgs;

      unfilteredRoot = ./.;
      coreFileset = craneLib.fileset.commonCargoSources unfilteredRoot;

      src = pkgs.lib.fileset.toSource {
        root = unfilteredRoot;
        fileset = coreFileset;
      };

      integrationTestFileset = pkgs.lib.fileset.unions [
        (unfilteredRoot + "/fixtures")
        (pkgs.lib.fileset.maybeMissing (unfilteredRoot + "/crates/grammar/tests/snapshots"))
      ];

      testSrc = pkgs.lib.fileset.toSource {
        root = unfilteredRoot;
        fileset = pkgs.lib.fileset.unions [
          coreFileset
          integrationTestFileset
        ];
      };

      commonArgs = {
        inherit src;
        strictDeps = true;
        buildInputs = [];
        nativeBuildInputs = with pkgs; [pkg-config];
      };

      cargoArtifacts = craneLib.buildDepsOnly commonArgs;

      fixtureArgs = commonArgs // {src = testSrc;};

      formatter =
        (
          treefmt-nix.lib.evalModule pkgs
          {
            projectRootFile = "flake.nix";
            settings.excludes = ["fixtures/*"];
            programs = {
              alejandra.enable = true;
              taplo.enable = true;
              rustfmt.enable = true;
              just.enable = true;
            };
          }
        )
        .config.build.wrapper;
    in {
      inherit formatter;

      packages = let
        cargoToml = fromTOML (builtins.readFile ./Cargo.toml);
        meta = cargoToml.workspace.metadata.crane or cargoToml.package;
        pname = meta.name;
        version = meta.version;
      in {
        default = craneLib.buildPackage (commonArgs
          // {
            inherit version cargoArtifacts pname;
            doCheck = false;
          });
      };

      checks = {
        inherit formatter;

        clippy = craneLib.cargoClippy (commonArgs
          // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--lib --bins -- -D warnings";
          });

        tests = craneLib.cargoNextest (fixtureArgs
          // {
            inherit cargoArtifacts;
            src = testSrc;
            nativeBuildInputs = commonArgs.nativeBuildInputs ++ [pkgs.cargo-nextest];
            env.INSTA_UPDATE = "no";
          });
      };

      devShells = let
        inherit pkgs formatter;
      in {
        default = pkgs.mkShell {
          packages = with pkgs; [
            just
            cargo-nextest
            cargo-insta
            cargo-expand
            formatter
          ];
        };
      };
    });
}
