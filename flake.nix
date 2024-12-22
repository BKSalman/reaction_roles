{
  description = "Reaction roles bot flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    nixpkgs,
    flake-utils,
    rust-overlay,
    crane,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: 
      let 
        craneLib = (crane.mkLib nixpkgs.legacyPackages.${system});

        pkgs = import nixpkgs { inherit system; overlays = [ rust-overlay.overlays.default ]; };

        buildInputs = with pkgs; [
          # needed for openssl dependant packages
          openssl
          pkg-config
        ];

        cargoArtifacts = craneLib.buildDepsOnly ({
          src = craneLib.cleanCargoSource (craneLib.path ./.);
          inherit buildInputs;
          pname = "reaction-roles";
        });
      in
    with pkgs; {
        packages = rec {
          reaction-roles = craneLib.buildPackage {
            src = craneLib.path ./.;

            inherit buildInputs cargoArtifacts;
          };

          default = reaction-roles;
        };

      devShell = mkShell.override {
          stdenv = pkgs.stdenvAdapters.useMoldLinker pkgs.clangStdenv;
        } {
          inherit buildInputs;

          packages = [
            # backend
            (rust-bin.stable.latest.default.override {
              extensions = [ "rust-src" "rust-analyzer" ];
            })
            diesel-cli

            # auto reload server on save
            # cargo watch -x run
            cargo-watch

            # database
            postgresql
            docker
          ];

          nativeBuildInputs = [
          ];

          shellHook = ''
            # run dev servers
            alias lcr='RUST_LOG=reaction_roles=debug,tower=debug cargo run'
            alias lcw='RUST_LOG=reaction_roles=debug,tower=debug cargo watch -x run'

            # start dev database if available, if not create, and run it on port 5445
            docker start reaction-roles-dev || \
              docker run \
              --name reaction-roles-dev \
              -p 5445:5432 \
              -e POSTGRES_PASSWORD=123 \
              -d postgres

            # add DATABASE_URL to .env file if not already there
            grep DATABASE_URL .env || echo "DATABASE_URL=postgres://postgres:123@localhost:5445/postgres" >> .env

            # export environment variables
            export $(cat .env)
          '';
        };

      formatter.x86_64-linux = legacyPackages.${system}.nixpkgs-fmt;
    });
}
