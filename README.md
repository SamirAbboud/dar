# dar

`dar` is a fast, lightweight local development environment manager written in Rust. It simplifies managing Nginx virtual hosts, PHP-FPM runtimes, and local `.test` domains.

---

## Features
<!-- TODO: Add feature details -->

---

## Installation
<!-- TODO: Add installation instructions -->

---

## Commands

### `install`
<!-- TODO: Add details for dar install -->

### `parked`
<!-- TODO: Add details for dar link -->

### `link`
<!-- TODO: Add details for dar link -->

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
      path = "${config.users.users.samir.home}/.local/share/mkcert/rootCA.pem";
      name = "mkcert-rootCA.pem";
    })
  ];
```

> **Flake Users:** When rebuilding your Flake configuration, pass `--impure` to allow Nix to read the certificate from your home directory:
>
> `sudo nixos-rebuild switch --flake .#nixos --impure`
