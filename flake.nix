{
  description = "picturium - a lightweight thumbnail server";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        picturium = pkgs.rustPlatform.buildRustPackage {
          pname = "picturium";
          version = "0.1.1";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
            outputHashes = { };
          };

          nativeBuildInputs = with pkgs; [ pkg-config ];

          buildInputs = with pkgs; [ vips ];

          doCheck = false;

          meta = with pkgs.lib; {
            description = "picturium - a lightweight thumbnail server";
          };

        };
      in {
        packages.default = picturium;

        nixosModules.picturium = { lib, ... }: {
          options = {
            services.picturium = with lib; {
              enable = mkOption {
                type = types.bool;
                default = false;
                description = "Enable the Picturium service.";
              };
              log = mkOption {
                type = types.str;
                default = "info";
                description = "Log level for Picturium. (debug, info, warn, error)";
              };
              host = mkOption {
                type = types.str;
                default = "127.0.0.1";
                description = "Host for Picturium.";
              };
              port = mkOption {
                type = types.int;
                default = 20045;
                description = "Port for Picturium.";
              };
              cors = mkOption {
                type = types.str;
                default = "";
                description = "CORS settings for Picturium.";
              };
              secret_key = mkOption {
                type = types.str;
                default = "";
                description = "A secret string used for HMAC verification in requests. Must be a secure, random string (e.g., 32+ chars)";
              };
              avifEnable = mkOption {
                type = types.bool;
                default = true;
                description = "Enable AVIF support in Picturium.";
              };
              cache = mkOption {
                type = types.str;
                default = "/var/cache/picturium";
                description = "Cache directory for Picturium.";
              };
              cacheEnable = mkOption {
                type = types.bool;
                default = true;
                description = "Enable caching in Picturium.";
              };
              cacheCapacity = mkOption {
                type = types.int;
                default = 10;
                description = "Cache capacity for Picturium.";
              };
              dataDir = mkOption {
                type = types.str;
                default = "/var/lib/picturium/data";
                description = "Data directory for Picturium.";
              };
            };
          };

          config = { config, pkgs, ... }: {
            systemd.services.picturium = {
              description = "Picturium Thumbnail Service";
              after = [ "network.target" ];
              wantedBy = [ "multi-user.target" ];
              serviceConfig = {
                ExecStart = "${pkgs.picturium}/bin/picturium";
                Environment = [
                  "LOG=${config.services.picturium.log}"
                  "HOST=${config.services.picturium.host}"
                  "PORT=${config.services.picturium.port}"
                  "CORS=${config.services.picturium.cors}"
                  "KEY=${config.services.picturium.secret_key}"
                  "AVIF_ENABLE=${config.services.picturium.avifEnable}"
                  "CACHE=${config.services.picturium.cache}"
                  "CACHE_ENABLE=${config.services.picturium.cacheEnable}"
                  "CACHE_CAPACITY=${config.services.picturium.cacheCapacity}"
                  "DATA_DIR=${config.services.picturium.dataDir}"
                ];
              };
            };
          };
        };

        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustc
            cargo
            rustfmt
            clippy
            rust-analyzer
            pkg-config
            vips
          ];
          LD_LIBRARY_PATH = "${pkgs.stdenv.cc.cc.lib}/lib:${pkgs.libexif}/lib";
        };
      });
}
