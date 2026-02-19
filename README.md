# Helioric ☀️

Screen brightness TUI written in Rust.

## Requisites

To talk to external monitors install [ddcutil](https://www.ddcutil.com/) and
enable [I2C](https://en.wikipedia.org/wiki/I2C). This may require adding
yourself to the `i2c` group which can be done with `hardware.i2c.enable = true`
on NixOS. For laptop screens only
[brightnessctl](https://github.com/Hummer12007/brightnessctl) is required.

## Installation

On NixOS, fetch `flake.nix` from GitHub and include it somewhere in your
configuration. Here's an example:

```nix
{ pkgs, ... }: let
  helioricFlake = builtins.getFlake "github:jameschubbuck/helioric";
in {
  environment.systemPackages = [
    helioricFlake.packages.${pkgs.system}.default
  ];
}
```

## Running on non-NixOS

A static binary is provided on the releases tab of this repository.
