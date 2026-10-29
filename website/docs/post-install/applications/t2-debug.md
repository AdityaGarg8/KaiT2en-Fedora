# T2 Debug

A live window into the T2's own operating system (bridgeOS), over the internal
T2 network link. It talks to the T2's built-in Instruments diagnostic
service and is read-only: it observes, it changes nothing.

Two tabs:

- **deviceinfo** is a "top" for the T2: every bridgeOS process with its CPU,
  power score, interrupt wakeups, memory footprint and disk writes, updated once
  a second. Click a column header to sort. Buttons run one-shot queries (mach
  time, network info, the kdebug trace-code table, the full service capability
  list) and list a bridgeOS directory.
- **live log** streams the T2's `os_log` in real time, decoded to readable
  lines and coloured by level like `dmesg`. A regex box filters, a "hide HID"
  toggle drops the input-device chatter, and the subsystem list on the right
  lets you tick exactly which subsystems to show.

This is the tool for watching what the T2 does in the moment, for example the
SMC and power events around a GPU switch or a suspend. Logs the T2 marks as
private appear as `<private>`: that redaction happens inside bridgeOS and cannot
be lifted from Linux.

A headless command, `t2debug`, exposes the same data without a GUI, which is
handy over SSH (`t2debug procs`, `t2debug logs --no-hid`,
`t2debug sysmon`). Run `t2debug --help` for all subcommands.
