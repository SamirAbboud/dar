# dar

`dar` is a fast, lightweight local development environment manager written in Rust. It simplifies managing Nginx virtual hosts, PHP-FPM runtimes, and local `.test` domains.

---

## Features

- ⚡ **Fast & Lightweight**: Built in Rust for maximum performance, single-binary efficiency, and instant startup times.
- 🌐 **Site Linking & Parking**: Link current directories or park entire dev folders (`dar link`, `dar parked`, `dar sites`) to automatically route local `.test` domains.
- 🔒 **Instant SSL/TLS**: Secure local sites with custom SSL certificates (`dar secure` / `dar unsecure`) for HTTPS support out of the box.
- 🐘 **Isolated PHP Runtimes**: Switch global PHP versions (`dar php -u`) or isolate individual project sites (`dar php -i`) to specific PHP-FPM runtimes or `.darrc` versions without system-wide conflicts.
- 📦 **PHP-Aware Composer**: Run `dar composer` using the exact PHP runtime configured for your active project.
- 🐳 **Containerized Services**: Provision background services (`dar services`) with dynamic port management, unified status tracking, and container-to-container DNS via a dedicated `dar-net` bridge network.
- ⚙️ **Extensible via User Templates**: Add custom Docker-based services or developer tools via `~/.dar/templates.toml` without altering core binaries.

---

## Installation

### 1. Prerequisites

Before installing `dar`, ensure you have the following installed and running:

- **Rust Toolchain**: Cargo / `rustup` installed to build and install the binary.
- **Docker**: Docker Engine / Docker Desktop running for containerized background services (`dar-net`).

### 2. Install the Binary

You can install `dar` directly using Cargo:

```bash
cargo install --git [https://github.com/SamirAbboud/dar.git](https://github.com/SamirAbboud/dar.git) --force --locked
```

>*Ensure `~/.cargo/bin` is in your system's `$PATH` so the `dar` executable can be accessed from anywhere.*

### 3. Initialize Environment

Once installed, initialize your local environment by running:

```bash
dar install
```

`dar install` bootstraps your entire local environment automatically:

1. **Directory Structure**: Initializes `~/.dar` configuration files, runtime directories, and log paths.
2. **Isolated Nginx**: Downloads, builds, and installs a standalone Nginx binary compiled directly from source with custom `pcre2`, `openssl`, and `zlib` dependencies.
3. **Capabilities & Security**: Grants required bind capabilities (`cap_net_bind_service`) to the Nginx binary so it can listen on privileged ports (80/443) without root access, and provisions `mkcert` for local SSL generation.
4. **PHP & Composer**: Downloads isolated PHP-FPM runtimes into `~/.dar` and installs `composer`.
5. **Services & Configuration**: Sets up base Nginx configuration files and launches the background daemon.

---

## Commands

### `install`

Initializes and bootstraps the complete local `dar` development environment from source and remote assets.

```bash
dar install
```

**What it does:**
- Downloads, compiles, and builds an isolated Nginx binary from source with custom dependencies (`pcre2`, `openssl`, `zlib`).
- Sets required network bind capabilities (`cap_net_bind_service`) on the Nginx executable so it can bind to ports 80 and 443 without root permissions.
- Provisions `mkcert` inside `~/.dar` for local root CA generation and SSL certificate management.
- Fetches isolated PHP-FPM runtimes and sets up `composer`.
- Pre-provisions `laravel/installer` inside `~/.dar/composer/` using the isolated Composer runtime.
- Initializes system directories, default configuration files, and launches the background daemon.

### `update`

Updates managed global tools and binaries isolated within the `~/.dar` environment.

```bash
# Update all managed binaries (Composer, Laravel Installer)
dar update

# Target a specific tool
dar update composer
dar update laravel
```

**What it does:**
- Checks and updates isolated global tools and binaries inside `~/.dar` without affecting your host system environment.
- Running `dar update composer` triggers Composer's native self-update mechanism (`composer self-update`).
- Running `dar update laravel` executes `composer global require laravel/installer:* -W` scoped to `~/.dar/composer/` using the isolated Composer runtime.
- Running `dar update` with no arguments sequentially updates all managed tools (`composer`, `laravel`).

### `parked`

Lists all registered parked development directories managed by `dar`.

```bash
dar parked
```

### `link`

Links a directory as a local `.test` site and generates its Nginx virtual host configuration.

```bash
# Link current working directory (uses folder name, e.g. folder 'my-app' -> [http://my-app.test](http://my-app.test))
dar link

# Specify a custom domain name
dar link --name custom-name

# Specify a target path explicitly
dar link --path /path/to/project

# Link and immediately secure with SSL/TLS
dar link --secure

# Combine flags
dar link --name custom-name --path /path/to/project --secure
```

| Flag | Short | Description |
| :--- | :--- | :--- |
| `--name` | `-n` | Custom domain name (defaults to target directory name) |
| `--path` | `-p` | Target directory path (defaults to current working directory) |
| `--secure` | `-s` | Provisions SSL/TLS certificate and links as HTTPS |


### `unlink`

Removes a linked local `.test` development site and deletes its Nginx virtual host configuration.

```bash
# Unlink current directory (defaults to current folder name)
dar unlink

# Unlink a specific site by domain name
dar unlink --name custom-name

# Unlink a site by specifying its target project path
dar unlink --path /path/to/project

# Combine flags
dar unlink --name custom-name --path /path/to/project
```

| Flag | Short | Description |
| :--- | :--- | :--- |
| `--name` | `-n` | Custom domain name of the site to unlink (defaults to target directory name) |
| `--path` | `-p` | Target directory path (defaults to current working directory) |


### `secure`

Provisions a local SSL/TLS certificate using `mkcert` and updates the Nginx virtual host configuration to enable HTTPS.

```bash
# Secure current directory site (defaults to current folder name)
dar secure

# Secure a site by domain name
dar secure --name custom-name

# Secure a site by specifying its target project path
dar secure --path /path/to/project

# Combine flags
dar secure --name custom-name --path /path/to/project
```

| Flag | Short | Description |
| :--- | :--- | :--- |
| `--name` | `-n` | Custom domain name of the site to secure (defaults to target directory name) |
| `--path` | `-p` | Target directory path (defaults to current working directory) |


### `unsecure`

Removes local SSL certificates for a site and reverts its Nginx virtual host configuration back to standard HTTP.

```bash
# Unsecure current directory site (defaults to current folder name)
dar unsecure

# Unsecure a site by domain name
dar unsecure --name custom-name

# Unsecure a site by specifying its target project path
dar unsecure --path /path/to/project

# Combine flags
dar unsecure --name custom-name --path /path/to/project
```

| Flag | Short | Description |
| :--- | :--- | :--- |
| `--name` | `-n` | Custom domain name of the site to unsecure (defaults to target directory name) |
| `--path` | `-p` | Target directory path (defaults to current working directory) |


### `php`

Manages isolated PHP runtimes, lists installed versions, and switches the active PHP version used by `dar`.

```bash
# Show the currently active PHP version (default when run without arguments)
dar php

# List all installed PHP versions
dar php --list

# Show the currently active PHP version
dar php --current

# Switch to or install a specific PHP version (e.g. 8.2)
dar php -u 8.2
dar php --use 8.3

# Isolate current project site to a specific PHP version or using .darrc
dar php -i
dar php -i 8.2
dar php --isolate 8.3

# Remove PHP isolation and restore project to global PHP-FPM
dar php -U
dar php --unisolate
dar php --un

# Pass arguments directly to the active PHP CLI binary
dar php -v
dar php -m
dar php -r 'echo "Hello from isolated PHP!";'
dar php -S localhost:8000 -t public/
```

| Flag / Argument | Short | Description |
| :--- | :--- | :--- |
| *(none)* | *(none)* | Display the currently active PHP version |
| `--list` | `-l` | List all installed PHP versions and their installation paths |
| `--current` | `-c` | Display the currently active PHP version |
| `--use <VER>` | `-u <VER>` | Switch to or provision a specific PHP version (e.g. `8.3`, `8.4`) |
| `--isolate <VER>` | `-i <VER>` | Isolate the current project site to a specific PHP version or using `.darrc` configuration |
| `--unisolate` | `-U`, `--un` | Remove PHP version isolation for the current project site and restore to global PHP-FPM |
| `<ARGS>...` | *(none)* | Pass raw CLI flags/arguments directly to the active `php` binary |


### `composer`

Executes Composer commands using the active isolated PHP runtime configured for `dar`.

```bash
# Run any standard Composer command
dar composer <args>

# Examples:
dar composer install
dar composer require laravel/framework
dar composer update --no-dev
```

| Flag/Argument | Description |
| :--- | :--- |
| `<args>` |Raw arguments passed directly to Composer (e.g. `require laravel/framework`) |


### `laravel`

Executes Laravel Installer commands using the active isolated PHP runtime configured for `dar`.

```bash
# Run any standard Laravel Installer command
dar laravel <args>

# Examples:
dar laravel --version
dar laravel new my-app
dar laravel new my-app --git --pest
```

| Flag/Argument | Description |
| :--- | :--- |
| `<args>` |Raw arguments passed directly to Laravel Installer (e.g. `new my-app`) |


### `sites`

Lists all active local development sites currently linked with `dar`.

```bash
dar sites
```

### `services`

Manages background infrastructure service containers on the `dar-net` Docker network.

```bash
# List available and running service containers
dar services --list

# Add and start a new service container
dar services --add mysql

# Start all core services (or a specific service)
dar services --start
dar services --start redis

# Stop all core services (or a specific service)
dar services --stop
dar services --stop redis

# Restart all core services (or a specific service)
dar services --restart
dar services --restart redis

# Remove a service container
dar services --remove mysql
```

| Flag | Short | Description |
| :--- | :--- | :--- |
| `--list` | `-l` | Lists available and running service containers |
| `--add <SERVICE>` | `-a` | Adds and starts a new service container |
| `--start [SERVICE]` | `-s` | Starts all core services or a specific service container |
| `--stop [SERVICE]` | `-t` | Stops all core services or a specific running service container |
| `--restart [SERVICE]` | `-r` | Restarts all core services or a specific service container |
| `--remove <SERVICE>` | `-d` | Removes a service container |


#### Adding Custom Services (`templates.toml`)

You can define additional service templates in your `templates.toml` file `~/.dar/templates.toml`. Below are pre-configured SQL Studio definitions for MySQL, PostgreSQL, and SQLite that you can copy to your setup:

```toml
[services.sqlstudio-mysql]
key = "sqlstudio-mysql"
name = "SQL Studio (MySQL)"
image = "frectonz/sql-studio:latest"
default_port = 3030
container_port = 3000
additional_ports = {}
env_vars = {}
volume_mounts = { data = "/data" }
command = [
    "/bin/sql-studio",
    "--no-browser",
    "--no-shutdown",
    "--address=0.0.0.0:3000",
    "mysql",
    "mysql://root:root@dar--mysql:3306/mysql"
]

[services.sqlstudio-pg]
key = "sqlstudio-pg"
name = "SQL Studio (PostgreSQL)"
image = "frectonz/sql-studio:latest"
default_port = 3031
container_port = 3000
additional_ports = {}
env_vars = {}
volume_mounts = { data = "/data" }
command = [
    "/bin/sql-studio",
    "--no-browser",
    "--no-shutdown",
    "--address=0.0.0.0:3000",
    "postgres",
    "postgres://postgres:postgres@dar--postgres:5432/postgres"
]

[services.sqlstudio-sqlite]
key = "sqlstudio-sqlite"
name = "SQL Studio (SQLite)"
image = "frectonz/sql-studio:latest"
default_port = 3032
container_port = 3000
additional_ports = {}
env_vars = {}
volume_mounts = { "/home/user/Sites" = "/sites" }
command = [
    "/bin/sql-studio",
    "--no-browser",
    "--no-shutdown",
    "--address=0.0.0.0:3000",
    "sqlite",
    "/sites/larademo/database/database.sqlite"
]
```
Once added to `templates.toml`, spin up any of the custom services with:

```bash
dar services --add sqlstudio-mysql
dar services --add sqlstudio-pg
dar services --add sqlstudio-sqlite
```

### Shell Auto-Completions

`dar` provides built-in auto-completion generation for major shells via `clap_complete`.

#### 1. Generate Completion Script

##### Fish
```fish
dar completions fish > ~/.config/fish/completions/dar.fish
```

##### Zsh
```zsh
mkdir -p ~/.zsh/completions
dar completions zsh > ~/.zsh/completions/_dar
```

##### Bash
```bash
mkdir -p ~/.local/share/bash-completion/completions
dar completions bash > ~/.local/share/bash-completion/completions/dar
```

---

## Local Domain Resolution (`.test`)

> **Note on `/etc/hosts`:** `dar` attempts to append entries to `/etc/hosts` automatically. On read-only filesystems (like NixOS), this step is safely skipped without throwing errors.

If `/etc/hosts` is read-only, you must configure local wildcard DNS once so all `*.test` domains automatically resolve to `127.0.0.1`.

### NixOS Configuration (`configuration.nix`)

```nix
  # Route ONLY .test queries to local dnsmasq
  services.resolved = {
    enable = true;
    settings = {
      Resolve = {
        DNS = [ "127.0.0.1:5353" ];
        Domains = [ "~test" ];
      };
    };
  };

  # Run dnsmasq on port 5353 to resolve *.test locally
  services.dnsmasq = {
    enable = true;
    settings = {
      port = 5353;
      listen-address = "127.0.0.1";
      address = [ "/.test/127.0.0.1" ];
    };
  };
```

### System-wide Trust for Local Root CA (`mkcert`)

On standard Linux distributions (Ubuntu, Arch, etc.), `dar` automatically installs the Root CA into your trust store. 

On **NixOS**, because `/etc/ssl/certs` is read-only, you must add your `mkcert` Root CA to `security.pki.certificateFiles` in `configuration.nix`:

```nix
  # Trust local mkcert Root CA system-wide
  security.pki.certificateFiles = [
    (builtins.path {
      path = "${config.users.users.<username>.home}/.local/share/mkcert/rootCA.pem";
      name = "mkcert-rootCA.pem";
    })
  ];
```

> **Flake Users:** When rebuilding your Flake configuration, pass `--impure` to allow Nix to read the certificate from your home directory:
>
> `sudo nixos-rebuild switch --flake .#nixos --impure`


---

## Global Environment & File Structure

`dar` manages its runtime state, configuration templates, isolated binaries, and TLS certificates inside `~/.dar`.

| Path | Description |
| :--- | :--- |
| `~/.dar/bin/` | System tools and binaries managed by `dar` (e.g., `composer`, `mkcert`) |
| `~/.dar/certs/` | Local SSL/TLS certificates generated by `mkcert` |
| `~/.dar/composer/` | Isolated `COMPOSER_HOME` housing globally installed Composer packages and binaries (e.g., `laravel/installer`) |
| `~/.dar/logs/` | Nginx, PHP-FPM, and background service log files |
| `~/.dar/nginx/` | Standalone Nginx build binaries and global server configuration files |
| `~/.dar/parked.txt` | Text file listing paths to registered parked development directories |
| `~/.dar/php/` | PHP environment directory containing global configs (`php.ini`, `php-fpm.conf`), active symlink (`current`), and installed runtimes under `versions/` |
| `~/.dar/services/` | Docker container volumes, configurations, and runtime states |
| `~/.dar/sites/` | Individual Nginx virtual host `.conf` files for linked `.test` domains |
| `~/.dar/sites.json` | JSON registry mapping linked site names, target project paths, SSL (`secured`) status, and creation timestamps |
| `~/.dar/isolations.json` | JSON registry mapping project paths to their isolated PHP runtime versions |
| `~/.dar/templates.toml` | Custom service templates and Docker container definitions |
| `~/.dar/versions.toml` | Pins default tool versions used during installation (`nginx`, `pcre2`, `openssl`, `zlib`, `default_php`, `mkcert`) |


