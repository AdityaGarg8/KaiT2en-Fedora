# t2-ave

The T2 video encoders live here: the userspace service, its protocol and the
kernel modules `t2bce_ave` and `t2bce_sidecar`. The existing `t2remote` command
and `kait2en-t2-remote.service` names remain available to preserve installed
configurations and suspend clients.

The parts follow the macOS components:

| macOS | here |
| --- | --- |
| `remoted` | `t2remote`, holds `com.apple.aveservice` and `com.apple.videoprocessingd.encode.remote` |
| `AppleAVEBridge` | `kernel/t2bce_ave`, AVE over BCE queues, V4L2 HEVC encoder `apple-ave-enc` |
| `VCPSideCarCompressionSession`, `VCPSideCarUSBSession` | `kernel/t2bce_sidecar`, V4L2 H.264/HEVC encoder `t2-sidecar` |

The T2 exposes the Sidecar encoder as a virtual USB device (`05ac:9312`, "SideCar
AVE") only while a client holds `com.apple.videoprocessingd.encode.remote`.
`t2remote` connects it at start after AVE, and `t2bce_sidecar` binds to the device
through `t2bce_vhci`. On macOS the Sidecar client runs in userspace inside
VideoProcessing.framework. Here it is a kernel driver, because a V4L2 encoder needs
one.

`daemon/` manages service sessions, `protocols/` owns the AVE start/stop messages,
and `integration/` contains the unit and the hook installed into the common
NCM suspend helper. NetworkManager and NCM binding belong to `../shared/`.

Build and install userspace with `make build` and `make install`. Use `PREFIX`
and `DESTDIR` for packaging. Building the kernel module requires matching kernel
headers and the BCE core's symbols. The Fedora DKMS stack stages the module from
`kernel/t2bce_ave/` alongside the core.
