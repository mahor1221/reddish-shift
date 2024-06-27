[![CICD](https://img.shields.io/github/actions/workflow/status/mahor1221/reddish-shift/cicd.yaml?style=for-the-badge&logo=githubactions)](https://github.com/mahor1221/reddish-shift/actions)
[![COVERAGE](https://img.shields.io/codecov/c/github/mahor1221/reddish-shift?style=for-the-badge&logo=codecov)](https://codecov.io/gh/mahor1221/reddish-shift)
[![CRATES.IO](https://img.shields.io/crates/v/reddish-shift.svg?style=for-the-badge&logo=rust)](https://crates.io/crates/reddish-shift)
[![REPOSITORIES](https://img.shields.io/repology/repositories/reddish-shift?style=for-the-badge)](https://repology.org/project/reddish-shift)

# Reddish-shift
A port of [Redshift](https://github.com/jonls/redshift).
Translated line by line with the help of [C2Rust](https://github.com/immunant/c2rust).

Reddish-shift adjusts the color temperature of your screen according to your
surroundings. This may help your eyes hurt less if you are working in front of
the screen at night.



## Installation
[![REPOSITORIES](https://repology.org/badge/vertical-allrepos/reddish-shift.svg?columns=3&exclude_unsupported=1)](https://repology.org/project/reddish-shift)

<details>
  <summary>Cargo</summary>

```bash
cargo install reddish-shift
```
</details>

<details>
  <summary>Archlinux</summary>

```bash
paru -S reddish-shift
paru -S reddish-shift-bin
paru -S reddish-shift-git
```
</details>


## Usage
For a quick start, run:
```bash
reddish-shift daemon --location LATITUDE:LONGITUDE
```
replace `LATITUDE` and `LONGITUDE` with your current geolocation.

To see all available commands:
```bash
reddish-shift -h
```

To see all available options for a given command (e.g. daemon):
```bash
reddish-shift daemon --help
```
Note that using `--help` instead of `-h` prints a more detailed help message.

A [configuration file](config.toml) can also be used. It should be saved in
the following location depending on the platform:
  * Linux: `$XDG_CONFIG_HOME/reddish-shift/config.toml`
           or `$HOME/.config/reddish-shift/config.toml` if `$XDG_CONFIG_HOME` is not set
           or `/etc/reddish-shift/config.toml` for system wide configuration
  * macOS: `$HOME/Library/Application Support/reddish-shift/config.toml`
  * Windows: `%AppData%\reddish-shift\config.toml`



## RoadMap
* Linux
  * [x] XRANDR gamma adjustment
  * [x] XVidMode gamma adjustment
  * [ ] DRM gamma adjustment
  * [ ] reddish-shift-gtk (from redshift-gtk)
  * [ ] systemd service, apparmor config (from [redshift/data](https://github.com/jonls/redshift/tree/master/data))
* Windows
  * [ ] Win32gdi gamma adjustment
* [ ] Support installation with: Appimage, AUR, DEB, PPA, MSI, Choco
* [ ] Geoclue2 location provider
* [ ] Real screen brightness control (experimental)
* Supporting macOS is not planned currently. Contributions are welcomed.
* [ ] Unit testing



## License
This project is licensed under the terms of [GNU General Public License v3.0](LICENSE).
