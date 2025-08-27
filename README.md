# hello-os

These instructions work on both Linux and macOS.

## Setup

### Install Nix

We use [Nix](https://nixos.org) to manage all dependencies.
Install Nix using the Determinate Systems installer which automatically sets up flakes and supports [easy uninstallation](https://github.com/DeterminateSystems/nix-installer#uninstalling):

```bash
curl -fsSL https://install.determinate.systems/nix | sh -s -- install
```

### Enter Nix Shell

Enter the development shell which includes all dependencies already set up:

```bash
nix develop
```

### Build & Run in QEMU

```bash
make run      # Graphical
make run-nox  # Non-graphical
```

### Attaching A Debugger

```bash
make gdb
```

## References

The baremetal Rust setup (features, linking, etc.) is best described in <https://os.phil-opp.com/set-up-rust/>.

A cleaner baremental setup (multi-boot and no dependencies on external tools): <https://kernelstack.net/2019-07-13-rust-os-1/>

Two versions of Philipp Oppermann blog: <https://os.phil-opp.com> (v2) and <https://os.phil-opp.com/first-edition> (v1)

Naked functions for exceptions: <https://os.phil-opp.com/first-edition/extra/naked-exceptions/>
