# ROG Control (Modern)

![screenshot](docs/screenshots/overview.png)
<!-- placeholder: docs/screenshots/{overview,fans,gpu,battery,keyboard}.png -->

## English

A modern Slint-based GUI for [asusd](https://github.com/OpenGamingCollective/asusctl) on ASUS ROG / TUF laptops: overview dashboard, performance profiles, fan curves, GPU mode, battery limit, keyboard lighting and more.

- Based on [OpenGamingCollective/asusctl](https://github.com/OpenGamingCollective/asusctl) (MPL-2.0). The daemon is **unchanged** and **required**: install and run `asusd` first.
- Not affiliated with or endorsed by ASUS.

### Install from release

```sh
tar xf asusctl-modern-ui-<version>-x86_64-linux.tar.gz
cd asusctl-modern-ui-<version>-x86_64-linux
./install.sh            # user-local, no sudo
./install.sh --uninstall
```

Installs to `~/.local/bin`, `~/.local/share/{applications,icons,locale}`. Launch "ROG Control (Modern)" from the app menu or run `rog-control-center-modern`.

### Build from source

Needs Rust (see `rust-toolchain.toml`), gettext (`msgfmt`) and the usual Slint/Wayland/X11 dev libraries.

```sh
cargo build --release -p rog-control-center-modern    # target/release/rog-control-center-modern
packaging/make-release.sh                      # tarball in dist/
```

### Coexistence with upstream rog-control-center

This app has its own identity, so both can be installed and run at the same time:

| | upstream | this fork |
|---|---|---|
| binary | `rog-control-center` | `rog-control-center-modern` |
| app id / D-Bus name | `xyz.ljones.rogcc` | `io.github.anilsrml.RogControlModern` |
| config | `~/.config/rog/` | `~/.config/rog-modern/` |
| gettext domain | `rog-control-center` | `rog-control-center-modern` |

Both talk to the same `asusd` (`xyz.ljones.*`). Avoid changing the same setting from both at once.

Translations are looked up in: `$ROGCC_LOCALE_DIR`, `<exe_dir>/../share/locale`, `$XDG_DATA_HOME/locale`, `/usr/share/locale`.

## Türkçe

ASUS ROG / TUF dizüstü bilgisayarlarda [asusd](https://github.com/OpenGamingCollective/asusctl) için Slint tabanlı modern bir arayüz: genel bakış, performans profilleri, fan eğrileri, GPU modu, pil sınırı, klavye aydınlatması ve fazlası.

- [OpenGamingCollective/asusctl](https://github.com/OpenGamingCollective/asusctl) (MPL-2.0) tabanlıdır. Servis (daemon) **değiştirilmemiştir** ve **gereklidir**: önce `asusd` kurulu ve çalışır olmalıdır.
- ASUS ile bağlantılı değildir, ASUS tarafından onaylanmamıştır.

### Sürümden kurulum

```sh
tar xf asusctl-modern-ui-<sürüm>-x86_64-linux.tar.gz
cd asusctl-modern-ui-<sürüm>-x86_64-linux
./install.sh            # kullanıcı dizinine, sudo gerekmez
./install.sh --uninstall
```

### Kaynaktan derleme

```sh
cargo build --release -p rog-control-center-modern
packaging/make-release.sh
```

### Upstream ile birlikte kullanım

Uygulamanın kendi kimliği vardır (`rog-control-center-modern`, D-Bus adı `io.github.anilsrml.RogControlModern`, ayarlar `~/.config/rog-modern/`); upstream `rog-control-center` ile aynı anda kurulu olabilir ve çalışabilir. İkisi de aynı `asusd` servisine bağlanır.

## License

MPL-2.0, see [LICENSE](LICENSE) and [NOTICE](NOTICE). Upstream README: [docs/UPSTREAM_README.md](docs/UPSTREAM_README.md).
