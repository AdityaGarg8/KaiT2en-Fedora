# KaiT2en Update

Experimental. The previous way of updating with `kait2en-install` keeps
working and stays the recommended one, see
[Updating](../updating.md).

Compares your installation with GitHub and installs only what changed. A full
run of the installer builds every app, every module and the initramfs again.
Most updates only touch one or two of them.

- Shows the local checkout, the state on GitHub and the new commits.
- Lists the installer steps and apps whose sources changed, each with a
  switch and the files that caused it. Unselected steps are offered again on
  the next check.
- Shows a restart hint when kernel modules or the initramfs were updated.
- Offers to install everything when the installer itself changed.
- Adds or removes the optional parts, T2 Touch Bar, Automatic brightness and
  KaiT2en Vornehm. A switch only marks the change, Update carries it out.

The installation runs through a restricted helper that asks for your password
and only accepts the names of known installer steps and apps.
