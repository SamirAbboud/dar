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
    domains = [ "~test" ];
    settings = {
      Resolve = {
        DNS = [ "127.0.0.1:5353" ];
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
