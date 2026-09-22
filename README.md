# ages

Show how old the people you care about are — live, in your terminal, with their photo.

```bash
$ ages list
alias  name               age                  zodiac      birthday
nana   Beatrix Lindqvist  81y 2m 19d 06:41:07  ♉ Taurus    birthday in 283 days
bro    Rafael Okonkwo     34y 0m 0d            ♎ Libra     🎂 birthday today
kiddo  Hana Petrova       2y 7m 12d            ♒ Aquarius  birthday in 199 days
```

Run `ages` in a terminal and you get an interactive view: the list on top, the selected person's
avatar and a per-second age counter below. Ages can be shown as a calendar breakdown or as total
years, months, weeks, days or hours. In a pipe it prints the table instead.

## Install

### Quick install (macOS and Linux)

Detects your OS and CPU, downloads the latest release, verifies its SHA-256 and installs to
`/usr/local/bin` (or `~/.local/bin` if that is not writable):

```bash
curl -fsSL https://raw.githubusercontent.com/eminaktas/ages/main/install.sh | sh
```

Pin a version or pick the install directory:

```bash
curl -fsSL https://raw.githubusercontent.com/eminaktas/ages/main/install.sh | AGES_VERSION=0.1.0 AGES_INSTALL="$HOME/bin" sh
```

### Install on macOS

1. Download the latest binary for your chip:

   ```bash
   # Apple Silicon
   curl -LO "https://github.com/eminaktas/ages/releases/latest/download/ages-macos-arm64.tar.gz"
   # Intel
   curl -LO "https://github.com/eminaktas/ages/releases/latest/download/ages-macos-x86_64.tar.gz"
   ```

2. Validate the archive (optional):

   ```bash
   curl -LO "https://github.com/eminaktas/ages/releases/latest/download/SHA256SUMS"
   shasum -a 256 --check --ignore-missing SHA256SUMS
   ```

3. Install:

   ```bash
   tar xzf ages-macos-*.tar.gz
   sudo install -m 755 ages /usr/local/bin/ages
   ```

4. Check: `ages --version`

### Install on Linux

1. Download the latest binary for your architecture (static musl build, works on any distro):

   ```bash
   # x86_64
   curl -LO "https://github.com/eminaktas/ages/releases/latest/download/ages-linux-x86_64.tar.gz"
   # arm64
   curl -LO "https://github.com/eminaktas/ages/releases/latest/download/ages-linux-arm64.tar.gz"
   ```

2. Validate the archive (optional):

   ```bash
   curl -LO "https://github.com/eminaktas/ages/releases/latest/download/SHA256SUMS"
   sha256sum --check --ignore-missing SHA256SUMS
   ```

3. Install:

   ```bash
   tar xzf ages-linux-*.tar.gz
   sudo install -m 755 ages /usr/local/bin/ages
   ```

4. Check: `ages --version`

### Build from source

Requires Rust 1.88+ (edition 2024).

```bash
cargo install --git https://github.com/eminaktas/ages   # latest main
cargo install --path .                                   # local checkout
```

## Quick start

```bash
ages add nana --name Beatrix --surname Lindqvist --birth 02.07.1945 --time 06:41 \
         --tz Europe/Stockholm --avatar ~/Pictures/nana.jpg
ages add bro --name Rafael --surname Okonkwo --birth 1992-09-21
ages add "little bro" --name Can --birth 2004-05-16 --avatar ~/Pictures/can.png --pixel
ages                # TUI (or a table when piped)
ages list --json    # machine-readable
```

Dates are `YYYY-MM-DD` or `DD.MM.YYYY`; time is `HH:MM` (24h). Without `--time` the age is shown
to the day. Without `--tz` the system timezone is used.

An alias is free text: any script, spaces and punctuation are fine (`Eşim`, `little bro`,
`anne (Ayşe)`), up to 64 characters. Only `/`, `\` and control characters are rejected, because
the alias doubles as the avatar file name. Quote aliases with spaces on the command line.

## Commands

```bash
ages                          TUI in a terminal, table in a pipe
ages list [--json] [--sort age|alias|birthday] [--view calendar|years|months|weeks|days|hours]
ages add <alias> --name <first> [--surname <last>] --birth <date>
         [--time HH:MM] [--tz <IANA>] [--avatar <image> [--pixel [GRID]]]
ages edit <alias> [--name ..] [--surname ..] [--birth ..] [--time ..] [--tz ..]
         [--avatar <image> [--pixel [GRID]]] [--no-avatar] [--rename <alias>]
ages remove <alias> [-y]
ages tui
Global: --lang en|tr   --data-dir <path>
```

Exit codes: 0 ok, 1 error, 2 usage.

## TUI keys

| key | action |
|---|---|
| `↑` `↓` `j` `k` | move |
| `a` | add person |
| `e` | edit person |
| `d` | delete (confirm with `y`) |
| `x` | remove the selected person's avatar (confirm with `y`) |
| `s` | sort picker: age / alias / birthday |
| `v` | age view picker: calendar / years / months / weeks / days / hours |
| `l` | language picker: English / Türkçe |
| `q` `Esc` | quit / close popup |

In the form: `Tab` / `Shift-Tab` move between fields, `Enter` saves, `Esc` cancels.
Pickers: `↑` `↓` choose, `Enter` selects, `Esc` cancels. Sort, view and language are saved to `people.toml`.

## Avatars

`--avatar` accepts PNG, JPEG, GIF or WebP. The image is center-cropped, resized to 256×256 and
stored under the data dir; the original is no longer needed. In terminals with a graphics
protocol (kitty, iTerm2, WezTerm, Ghostty, sixel-capable ones) it renders as real pixels;
elsewhere it falls back to half-block characters. Set `AGES_HALFBLOCKS=1` to skip the terminal
capability query and always use half-blocks.

Remove an avatar with `ages edit <alias> --no-avatar` or `x` in the TUI.

### Pixel art

Add `--pixel` next to `--avatar` to store the avatar as pixel art instead of a photo:

```bash
ages add nana --name Beatrix --birth 02.07.1945 --avatar nana.jpg --pixel      # 32×32 mosaic
ages edit nana --avatar nana.jpg --pixel 48                                      # finer grid
ages edit nana --avatar nana.jpg                                                 # back to a photo
```

The conversion happens once, at import: the image is center-cropped, box-downsampled to a
`GRID×GRID` mosaic (default 32, 2–256), reduced to an adaptive 32-color palette and stored as a
hard-edged 256×256 PNG. Nothing is recomputed at render time, so the TUI is as fast as with photos.
In the TUI form the last field asks `pixel art? (y/n or grid size)`. `people.toml` records
`pixel = true` so the viewer keeps the edges crisp. The conversion lives in the small
[`pixelart`](crates/pixelart) crate in this repo (built on `image` + `color_quant`).

## Data

Everything lives in `~/.ages/`:

```bash
~/.ages/people.toml          # people + settings (lang, sort, age_view); plain TOML, edit freely
~/.ages/avatars/<alias>.png
```

Override the location with `--data-dir <path>` or `AGES_HOME=<path>`.

## Language

`--lang` beats `settings.lang` in `people.toml`, which beats your desktop locale (`LANG`).
English is the fallback. Translations live in `i18n/{en,tr}/ages.ftl` (Fluent) and are
compiled into the binary.

## Development

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The repo is a Cargo workspace: the `ages` binary at the root and `crates/pixelart`, the
photo-to-pixel-art library it depends on.

CI (`.github/workflows/ci.yml`) runs fmt, clippy and tests on Linux and macOS for every push to
`main` and every pull request.

### Releasing

1. Bump `version` in `Cargo.toml` (and `Cargo.lock` via `cargo update -p ages`), commit.
2. Tag with the bare version and push: `git tag 0.1.0 && git push origin main 0.1.0`.

`.github/workflows/release.yml` checks that the tag equals the crate version, builds four
targets (`ages-macos-arm64`, `ages-macos-x86_64`, `ages-linux-x86_64`, `ages-linux-arm64`;
Linux ones are static musl binaries) and publishes a GitHub Release with the `.tar.gz` archives
and a `SHA256SUMS` file. `install.sh` reads exactly those assets.
