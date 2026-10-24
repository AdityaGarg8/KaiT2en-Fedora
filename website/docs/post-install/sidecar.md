# Streaming Using Sidecar

With the T2's Sidecar encoder, the desktop can be streamed to an Android tablet,
iPad, a phone or another computer. [Sunshine](https://github.com/LizardByte/Sunshine)
runs on the Mac and encodes with the T2, and
[Moonlight](https://moonlight-stream.org) shows the stream on the other device
and sends touch, mouse and keyboard input back. Thanks to the T2 being an ARM
processor, this works significantly faster than on the x86 CPU while saving plenty
of power.

The encoder is described in
[Hardware video encoding](hardware-video-encoding.md#sidecar-encoder).

## Sunshine with V4L2 support

Released Sunshine versions cannot use V4L2 encoders yet. The support is in
[pull request #5717](https://github.com/LizardByte/Sunshine/pull/5717), which
was opened in September 2026 for the Raspberry Pi and is still under review.
Until it is merged, the `t2-sidecar` branch of
[our Sunshine fork](https://github.com/deqrocks/Sunshine) carries that pull
request unchanged. Nothing in it is specific to the T2. Once the pull request
is merged, the regular Sunshine packages will work and this fork goes away.

The pull request encodes without copies. The GPU converts every captured frame
straight into the encoder's input buffers.

Install our Sunshine fork with

```bash
sudo dnf install git cmake ninja-build gcc gcc-c++ glslc npm python3-jinja2 desktop-file-utils libappstream-glib appstream boost-devel boost-static libcap-devel libcurl-devel libdrm-devel libevdev-devel libva-devel libX11-devel libxcb-devel libXcursor-devel libXfixes-devel libXi-devel libXinerama-devel libXrandr-devel libXtst-devel mesa-libGL-devel mesa-libgbm-devel miniupnpc-devel numactl-devel openssl-devel opus-devel pipewire-devel pulseaudio-libs-devel qt6-qtbase-devel qt6-qtsvg-devel vulkan-loader-devel
git clone --branch t2-sidecar --recurse-submodules --shallow-submodules https://github.com/deqrocks/Sunshine.git
cd Sunshine
cmake -B build -G Ninja -DCMAKE_BUILD_TYPE=Release -DBUILD_DOCS=OFF -DBUILD_TESTS=OFF -DSUNSHINE_ENABLE_CUDA=OFF -DSUNSHINE_ASSETS_DIR=share/sunshine
ninja -C build
sudo cmake --install build
```

## Configuration

Put this into `~/.config/sunshine/sunshine.conf`:

```ini
encoder = v4l2m2m
capture = portal
hevc_mode = 1
av1_mode = 1
system_tray = disabled
```

- `encoder = v4l2m2m` selects the V4L2 encoder.
- `hevc_mode = 1` and `av1_mode = 1` keep the stream on H.264. With HEVC,
  ffmpeg may pick the AVE encoder instead of `t2-sidecar`.
- `capture = portal` records the screen through GNOME's screen sharing.
- `system_tray = disabled` is needed on GNOME. Without a tray, Sunshine
  otherwise ends right after starting.

Start `sunshine` from a terminal. On the first start GNOME asks which screen
to share. Pick the built-in display. Sunshine remembers the choice and restarts
itself. Then open <https://localhost:47990>, create a user name and password,
and pair Moonlight with the PIN it shows.

## Extended display

To use the tablet as a second screen, switch on the Sidecar display:

```bash
t2-sidecar on
```

GNOME then shows an additional monitor next to the built-in display. Arrange it,
set its resolution or switch to mirroring in Settings > Displays like any other
monitor. Pick the same resolution as in Moonlight, otherwise Sunshine scales
every frame and the picture gets less sharp. The next `t2-sidecar on` starts
with the resolution picked last.

With Sunshine installed to `/usr/local/bin/sunshine` as described above,
`t2-sidecar on` also starts Sunshine, and GNOME asks which screen to share. Pick
the new monitor. Sunshine runs only while the Sidecar display is on, so GNOME's
screen sharing indicator is not shown otherwise. Do not start `sunshine` from a
terminal at the same time.

`t2-sidecar off` stops Sunshine and removes the monitor. Windows on it move back
to the built-in display. Locking the screen and logging out switch it off as
well.

Every `t2-sidecar on` creates a new virtual monitor, and GNOME does not apply a
saved screen choice to it. That is why the dialog comes up each time.

## Moonlight settings

- Resolution 1080p at 60 frames per second should run well on most modern WiFi
  setups. While testing, we managed around 50 to 60FPS in 2560*1600 using WiFi6
  and a rather modern Android tablet with Snapdragon CPU.
- Video codec H.264 or automatic.
- Sound plays on the tablet while streaming. To keep it on the Mac, enable
  playing audio on the PC in Moonlight's settings. To never stream sound from
  any device, add `stream_audio = disabled` to `sunshine.conf`.

## Limitations

- GNOME asks which screen to share again after every `t2-sidecar on`, see
  [Extended display](#extended-display).
- Locking the screen ends the Sidecar display, GNOME stops all screen sharing
  then. Switch it on again after unlocking.
- Sunshine logs "Encoder did not produce IDR frame when requested" while
  streaming. The encoder returns frames asynchronously and Sunshine checks the
  wrong packet. Keyframes on request arrive on the requested frame. Seems to be
  a cosmetic issue only.
