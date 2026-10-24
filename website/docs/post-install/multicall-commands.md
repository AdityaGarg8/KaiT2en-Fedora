# Multicall commands

KAIT2EN installs a helper script, `kait2en-multicall`, and links several short
commands to it. The command you type decides what runs. All of them are meant
to be started as your normal user without sudo. They ask for administrator
access when needed.

| Command | What it does |
| --- | --- |
| `edit-grub` | Opens `/etc/default/grub` in nano as root |
| `update-grub` | Regenerates `/boot/grub2/grub.cfg` |
| `boot-macos` | Restarts into macOS once. Linux is the default again on the next boot |
| `update-modules` | Updates the KAIT2EN checkout, then reinstalls the DKMS modules, rebuilds the patched AMDGPU driver on supported models and rebuilds the initramfs |
| `update-apps` | Updates the KAIT2EN checkout, then reinstalls the KAIT2EN applications |

## Incremental updates

`update-modules` and `update-apps` update only a part of the installation. They
are faster than `kait2en-install`, which runs all steps. Both reset the clean
checkout in `/usr/local/src/KaiT2en-Fedora` to the `main` branch of the remote
first, so they need the checkout that `kait2en-install` created.

```bash
update-modules
update-apps
```

Reboot after `update-modules` to load the updated modules. The AMDGPU step is
skipped on Macs that do not use the patched driver.

Run `kait2en-install` for everything else, see [Updating](updating.md).
