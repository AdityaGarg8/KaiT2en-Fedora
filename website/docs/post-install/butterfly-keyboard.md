# Butterfly keyboard debounce

Butterfly keyboards sometimes register one keystroke twice, so a word like
"unwanted" comes out as "unwaanteed". `t2hid` can filter these repeats. It drops
a key press that follows the release of the same key within a configurable
number of milliseconds, together with the matching release.

The filter only applies to butterfly keyboards:

| Model | Identifier |
| --- | --- |
| MacBook Pro 13" 2018/2019, four Thunderbolt ports | MacBookPro15,2 |
| MacBook Pro 13" 2019, two Thunderbolt ports | MacBookPro15,4 |
| MacBook Pro 15" 2018/2019 | MacBookPro15,1 and MacBookPro15,3 |
| MacBook Air 2018/2019 | MacBookAir8,1 and MacBookAir8,2 |

It is off by default.

## Enable it

Try it at runtime first:

```bash
echo 40 | sudo tee /sys/module/t2hid/parameters/debounce_ms
```

`0` turns it off again. To keep it across reboots, add the kernel argument:

```bash
sudo grubby --update-kernel=ALL --args="t2hid.debounce_ms=40"
```

## Tune the interval

40 ms is a good start. Raise the value if keys still repeat, lower it if
quickly typed double letters such as "ee" lose a letter. Every dropped press is
logged with its interval once dynamic debug is enabled for `t2hid`:

```bash
echo 'module t2hid +p' | sudo tee /sys/kernel/debug/dynamic_debug/control
sudo dmesg -w | grep debounce
```

A key that bounces when pressed and is then held down produces one character
and no key repeat.
