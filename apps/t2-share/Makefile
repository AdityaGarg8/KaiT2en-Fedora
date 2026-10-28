CARGO ?= cargo
INSTALL ?= install
GTK_UPDATE_ICON_CACHE ?= gtk-update-icon-cache

APP_ID := org.t2share.gtk
BIN_NAME := t2-share
CLI_NAME := t2share
BUILD_DIR := $(or $(CARGO_TARGET_DIR),target)/release
BIN_SRC := $(BUILD_DIR)/$(BIN_NAME)
CLI_SRC := $(BUILD_DIR)/$(CLI_NAME)
DESKTOP_SRC := $(APP_ID).desktop
ICON_SRC := assets/icons/hicolor/scalable/apps/$(APP_ID).svg
UNIT_SRC := systemd/t2-share.service

PREFIX ?= /usr/local
BIN_DIR := $(PREFIX)/bin
APPS_DIR := $(PREFIX)/share/applications
ICONS_DIR := $(PREFIX)/share/icons/hicolor/scalable/apps
ICON_THEME_DIR := $(PREFIX)/share/icons/hicolor
USER_UNIT_DIR := $(PREFIX)/lib/systemd/user
LICENSE_DIR := $(PREFIX)/share/licenses/t2-share

.PHONY: build check-build install uninstall clean

build:
	$(CARGO) build --release --locked

check-build:
	@test -x "$(BIN_SRC)" -a -x "$(CLI_SRC)" || { echo "error: $(BIN_SRC) or $(CLI_SRC) is missing; run 'make build' as your normal user first" >&2; exit 1; }

install: check-build
	$(INSTALL) -d -o root -g root -m 0755 $(BIN_DIR) $(APPS_DIR) $(ICONS_DIR) $(ICON_THEME_DIR) $(USER_UNIT_DIR) $(LICENSE_DIR)
	$(INSTALL) -o root -g root -m 0755 $(BIN_SRC) $(BIN_DIR)/$(BIN_NAME)
	$(INSTALL) -o root -g root -m 0755 $(CLI_SRC) $(BIN_DIR)/$(CLI_NAME)
	$(INSTALL) -o root -g root -m 0644 $(DESKTOP_SRC) $(APPS_DIR)/$(APP_ID).desktop
	$(INSTALL) -o root -g root -m 0644 $(ICON_SRC) $(ICONS_DIR)/$(APP_ID).svg
	$(INSTALL) -o root -g root -m 0644 $(UNIT_SRC) $(USER_UNIT_DIR)/t2-share.service
	$(INSTALL) -o root -g root -m 0644 THIRD-PARTY-NOTICES.md proto/LICENSE-Apache-2.0 proto/LICENSE-Chromium $(LICENSE_DIR)/
	$(GTK_UPDATE_ICON_CACHE) --force --ignore-theme-index $(ICON_THEME_DIR)

uninstall:
	rm -f $(BIN_DIR)/$(BIN_NAME)
	rm -f $(BIN_DIR)/$(CLI_NAME)
	rm -f $(APPS_DIR)/$(APP_ID).desktop
	rm -f $(ICONS_DIR)/$(APP_ID).svg
	rm -f $(USER_UNIT_DIR)/t2-share.service
	rm -rf $(LICENSE_DIR)
	@if [ -d "$(ICON_THEME_DIR)" ]; then $(GTK_UPDATE_ICON_CACHE) --force --ignore-theme-index $(ICON_THEME_DIR); fi

clean:
	$(CARGO) clean
