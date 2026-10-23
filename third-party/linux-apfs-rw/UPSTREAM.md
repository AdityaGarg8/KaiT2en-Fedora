# linux-apfs-rw

Source: https://github.com/linux-apfs/linux-apfs-rw

Commit: `03187127cd2e62cf78321cdb942eb3a7b17981f9`

License: GPL-2.0-only, see `LICENSE`. Files under `lzfse/` are BSD-3-Clause
(Apple Inc.), `libzbitmap.c` and `libzbitmap.h` are GPL-2.0-or-later OR MIT.
Each file carries its `SPDX-License-Identifier`.

This copy carries no Kait2en changes. `.gitignore` is left out.

The installer registers it with DKMS as `linux-apfs-rw` and installs the
`apfs` module. Mounts are read-only unless `-o readwrite` is passed.
