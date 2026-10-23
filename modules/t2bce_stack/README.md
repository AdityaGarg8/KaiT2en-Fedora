# t2bce_stack

This is the DKMS build unit for the tightly coupled T2 BCE driver stack. It
builds `t2bce_dma`, `t2bce_core`, `t2bce_vhci`, `t2bce_audio`, `t2bce_ave` and
`t2bce_sidecar` together so Kbuild and modpost share one symbol-version
namespace. The installer stages AVE and Sidecar from
`t2-services/t2-ave/kernel/`. The other members remain under `modules/`. AVE
source ownership does not change the shared DKMS build or module names.
`t2bce_sidecar` is a USB driver for a device `t2bce_vhci` exposes and uses no BCE
core symbols.

The individual source directories remain independently buildable for driver
development. They are staged below this directory by the Fedora DKMS installer.
