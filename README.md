[![CI](https://img.shields.io/badge/github%20actions-%232671E5.svg?style=for-the-badge&logo=githubactions&logoColor=white)](https://github.com/mahor1221/reddish-shift/actions)
[![Crates.io](https://img.shields.io/crates/v/reddish-shift.svg?style=for-the-badge)](https://crates.io/crates/reddish-shift)



# Reddish-shift
A port of [Redshift](https://github.com/jonls/redshift)
Translated line by line with the help of [C2Rust](https://github.com/immunant/c2rust)

Reddish-shift adjusts the color temperature of your screen according to your
surroundings. This may help your eyes hurt less if you are working in front of
the screen at night.

![demo](../media/demo.png?raw=true)



## Installation
[![Packaging status](https://repology.org/badge/vertical-allrepos/reddish-shift.svg?columns=3&exclude_unsupported=1)](https://repology.org/project/reddish-shift)

* Cargo:
```shell
cargo install reddish-shift
```


## Usage
* For a quick start, run:
```shell
reddish-shift daemon --location LATITUDE:LONGITUDE
```
replace `LATITUDE` and `LONGITUDE` with your current geolocation.

* To see all available commands:
```shell
reddish-shift -h
```

* To see all available options for a given command (e.g. daemon):
```shell
reddish-shift daemon --help
```

Note that using `--help` instead of `-h` prints a more detailed help message.

* A [configuration file](config.toml) can also be used. It should be saved in
the following location depending on the platform:
  * Linux: `$XDG_CONFIG_HOME/reddish-shift/config.toml`
           or `$HOME/.config/reddish-shift/config.toml` if `$XDG_CONFIG_HOME` is not set
           or `/etc/reddish-shift/config.toml` for system wide configuration
  * macOS: `$HOME/Library/Application Support/reddish-shift/config.toml`
  * Windows: `%AppData%\reddish-shift\config.toml`



## RoadMap
* Linux support
  * [x] XRANDR
  * [x] XVidMode
  * [ ] DRM
  * [ ] reddish-shift-gtk (from redshift-gtk)
  * [ ] systemd service, apparmor config (from redshift/data)
  * packages
    * [x] AUR
    * [ ] Appimage
    * [ ] DEB
    * [ ] PPA
* Windows support
  * [ ] Win32gdi
  * [ ] MSI package
  * [ ] Choco package
* Supporting macOS is not planned currently. Contributions are welcomed.
* [ ] Geoclue2 location provider
* [ ] Real screen brightness control (experimental)



## License
This project is licensed under the terms of [GNU General Public License v3.0]
(LICENSE).
