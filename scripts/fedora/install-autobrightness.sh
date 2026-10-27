#!/usr/bin/env bash
# Installs automatic brightness, or removes it again with --remove.
source "$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)/lib.sh"
require_root
require_repo_root
require_fedora

COMPONENT="$REPO_ROOT/t2-services/t2-autobrightness"
UNIT=kait2en-t2-autobrightness.service
EXTENSION=kait2en-autobrightness@kait2en

# Runs a command in the running session of the invoking user. Without a
# session there is nothing to do now, the next login picks up the change.
in_session() {
	local user="${SUDO_USER:-}" uid runtime
	[[ -n "$user" && "$user" != root ]] || return 0
	uid=$(id -u "$user")
	runtime=/run/user/$uid
	[[ -S "$runtime/bus" ]] || return 0
	runuser -u "$user" -- env XDG_RUNTIME_DIR="$runtime" DBUS_SESSION_BUS_ADDRESS="unix:path=$runtime/bus" \
		bash -c "$1"
}

build_autobrightness() {
	local target_user="${SUDO_USER:-}"
	[[ -n "$target_user" && "$target_user" != root ]] || fail "t2-autobrightness must be built by the invoking user"
	build_cargo_project "$COMPONENT" "$target_user"
}

# A user service, started with every graphical session. On Macs without the
# sensor or without a built-in panel it exits right away.
install_autobrightness() {
	make -C "$COMPONENT" install
	systemctl --global enable "$UNIT"
}

# Stopping the service gives GNOME its brightness settings back.
remove_autobrightness() {
	in_session "systemctl --user disable --now $UNIT; gnome-extensions disable $EXTENSION 2>/dev/null; true"
	systemctl --global disable "$UNIT"
	make -C "$COMPONENT" uninstall
	in_session "systemctl --user daemon-reload"
}

if [[ ${1:-} == --remove ]]; then
	run_step "remove t2-autobrightness" remove_autobrightness
	exit 0
fi

run_step "build t2-autobrightness" build_autobrightness
if (( STEP_STATUS == 0 )); then
	run_step "install t2-autobrightness" install_autobrightness
	(( STEP_STATUS == 0 )) &&
		run_step "restart t2-autobrightness in the running session" \
			in_session "systemctl --user daemon-reload && systemctl --user restart $UNIT"
else
	record_error "automatic brightness skipped because its build failed; existing installation retained"
fi
