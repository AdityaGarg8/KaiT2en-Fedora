#!/usr/bin/env bash

source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)/lib.sh"

require_root
require_repo_root
require_fedora
require_command systemctl install

# The apfs module from third-party/linux-apfs-rw mounts one volume of the
# container at a time and the volume order differs between Macs and macOS
# versions. The helper therefore probes every volume of each internal APFS
# container on every boot and mounts the readable ones read-only under
# /media, named by the role it detects from the contents. The encrypted Data
# volume is skipped.
info "installing Kait2en macOS volume mount"
install -d -o root -g root -m 0755 /usr/local/libexec/kait2en
tee /usr/local/libexec/kait2en/kait2en-macos-mount >/dev/null <<'EOF'
#!/bin/sh

# Mount points by detected role. The System volume keeps the plain /media/macOS
# name because it is the one users browse.
SYSTEM_TARGET=/media/macOS
PREBOOT_TARGET=/media/macOS-Preboot
ISCPREBOOT_TARGET=/media/macOS-iSCPreboot
RECOVERY_TARGET=/media/macOS-Recovery

ALL_TARGETS="$SYSTEM_TARGET $PREBOOT_TARGET $ISCPREBOOT_TARGET $RECOVERY_TARGET"

case "$1" in
stop)
	for t in $ALL_TARGETS; do
		mountpoint -q "$t" && umount "$t"
	done
	exit 0
	;;
esac

modprobe apfs || exit 1

# Print the role of the volume mounted read-only at $1, or nothing.
detect_role() {
	m=$1
	if [ -f "$m/System/Library/CoreServices/SystemVersion.plist" ]; then
		echo system
	elif [ -f "$m/nvram.plist" ] && [ -d "$m/Firmware" ]; then
		echo iscpreboot
	elif ls "$m"/*/restore/BuildManifest.plist >/dev/null 2>&1 ||
		ls -d "$m"/*/cryptex1 >/dev/null 2>&1; then
		echo preboot
	elif ls "$m"/*/*.im4m >/dev/null 2>&1; then
		echo recovery
	fi
}

target_for_role() {
	case $1 in
	system) echo "$SYSTEM_TARGET" ;;
	preboot) echo "$PREBOOT_TARGET" ;;
	iscpreboot) echo "$ISCPREBOOT_TARGET" ;;
	recovery) echo "$RECOVERY_TARGET" ;;
	esac
}

probe=$(mktemp -d /run/kait2en-macos-mount.XXXXXX) || exit 1
mounted_any=0

# Internal APFS partitions only. External disks are left to the file manager,
# and loop devices (mounted dmgs) are skipped so manual experiments are not
# touched.
for dev in $(lsblk -lnpo PATH,FSTYPE,RM,TYPE |
	awk '$2 == "apfs" && $3 == "0" && $4 == "part" { print $1 }'); do
	vol=0
	while [ "$vol" -le 9 ]; do
		if mount -t apfs -o ro,vol="$vol" "$dev" "$probe" 2>/dev/null; then
			role=$(detect_role "$probe")
			umount "$probe"
			target=$(target_for_role "$role")
			if [ -n "$target" ] && ! mountpoint -q "$target"; then
				mkdir -p "$target"
				if mount -t apfs -o ro,vol="$vol" "$dev" "$target"; then
					mounted_any=1
				fi
			fi
		fi
		vol=$((vol + 1))
	done
done

rmdir "$probe"
[ "$mounted_any" = 1 ] || echo "no readable macOS volume found" >&2
exit 0
EOF
chmod 0755 /usr/local/libexec/kait2en/kait2en-macos-mount

tee /etc/systemd/system/kait2en-macos-mount.service >/dev/null <<'EOF'
[Unit]
Description=Kait2en mount the readable macOS volumes read-only
After=local-fs.target

[Service]
Type=oneshot
RemainAfterExit=yes
ExecStart=/usr/local/libexec/kait2en/kait2en-macos-mount
ExecStop=/usr/local/libexec/kait2en/kait2en-macos-mount stop

[Install]
WantedBy=multi-user.target
EOF
chmod 0644 /etc/systemd/system/kait2en-macos-mount.service

systemctl daemon-reload
systemctl enable kait2en-macos-mount.service

info "Kait2en macOS volume mount installed"
