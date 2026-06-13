# Installing poneglyph

## From source

```bash
git clone https://github.com/ShamanicArts/poneglyph.git
cd poneglyph || exit
cargo install --path .
```

Then run:

```bash
poneglyph README.md
```

## From GitHub Releases

Download the archive for your platform from the Releases page, extract it, and put the `poneglyph` binary somewhere on your `PATH`.

Suggested install location:

```bash
mkdir -p ~/.local/bin
tar -xzf poneglyph-*-linux-x86_64.tar.gz
mv poneglyph ~/.local/bin/
```

## From crates.io

Once published:

```bash
cargo install poneglyph
```

##Nix (flakes)

Add the zarumet repo as a flake input

```nix
{
  inputs = {
    zarumet = {
      url = "github:Immelancholy/zarumet";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
}
```

Add to your package list

```nix
{ pkgs, inputs, ... }:
{
  environment.systemPakcages = [
    inputs.poneglyph.packages.${pkgs.stdenv.hostPlatform.system}.default
  ];
}
```

Remember to add inputs to specialArgs!

```nix
nixpkgs.lib.nixosSystem {
  specialArgs = {
    inherit
    inputs;
  };
```

Alternatively you can import the home-manager module into home manager and enable the program with.

```nix
{
  programs.poneglyph.enable = true;
}
```

## Configuration

User config:

```text
~/.config/poneglyph/config.toml
```

Project config:

```text
.poneglyph.toml
```

Example:

```toml
[ui]
theme = "tokyo-night"
cursorStyle = "block"
boxedChrome = true
themeSwatches = "square"
themeSwatchSpacing = 0
```
