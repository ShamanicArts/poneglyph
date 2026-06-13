self:
{
  lib,
  pkgs,
  config,
  ...
}:
with lib;
let
  cfg = config.programs.poneglyph;
  poneglyph = self.packages.${pkgs.stdenv.hostPlatform.system}.default;

  tomlFormat = pkgs.formats.toml { };
in
{
  options.programs.poneglyph = {
    enable = mkEnableOption "poneglyph";
    package = mkOption {
      type = types.package;
      default = poneglyph;
      description = "The poneglyph package to use";
    };

    settings = mkOption {
      type = tomlFormat.type;
      default = { };
      example = literalExpression ''
        {
          ui = {
            theme = "tokyo-night";
            cursorStyle = "block";      # brackets | block | bar | underline | box
            boxedChrome = true;
            themeSwatches = "square";   # off | circle | square
            themeSwatchSpacing = 0;     # 0..8
          };
        }
      '';
      description = "Settings for poneglyph";
    };
  };
  config = mkIf cfg.enable {
    home.packages = mkIf (cfg.package != null) [ cfg.package ];

    xdg.configFile."poneglyph/config.toml" = mkIf (cfg.settings != { }) {
      source = tomlFormat.generate "config.toml" cfg.settings;
    };
  };
}
