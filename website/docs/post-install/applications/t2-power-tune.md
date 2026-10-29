# T2 Power Tune

Scans for available PCIe ASPM, runtime power-management, wakeup, LTR, and other
power-saving tunables. Selected changes can be tested temporarily or installed
as a persistent systemd service. Replaces powertop/tlp for reaching deeper
(pkg) c-states. Do not change anything without reason. Enabling more options
does not mean more savings. It is more likely to cause regressions.
