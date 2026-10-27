CARGO ?= cargo
CARGO_FLAGS ?= --locked
PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
SYSTEMD_USER_UNIT_DIR ?= $(PREFIX)/lib/systemd/user
DATADIR ?= $(PREFIX)/share
EXTENSION_UUID = kait2en-autobrightness@kait2en
TARGET_DIR ?= $(or $(CARGO_TARGET_DIR),target)

.PHONY: all build test install install-bin install-integration uninstall clean
all: build
build:
	$(CARGO) build --release $(CARGO_FLAGS) --target-dir $(TARGET_DIR)
test:
	$(CARGO) test $(CARGO_FLAGS) --target-dir $(TARGET_DIR)
install: install-bin install-integration
install-bin:
	install -D -m 0755 $(TARGET_DIR)/release/t2-autobrightness $(DESTDIR)$(BINDIR)/t2-autobrightness
install-integration:
	install -d $(DESTDIR)$(SYSTEMD_USER_UNIT_DIR)
	sed 's|@BINDIR@|$(BINDIR)|g' integration/systemd/kait2en-t2-autobrightness.service >$(DESTDIR)$(SYSTEMD_USER_UNIT_DIR)/kait2en-t2-autobrightness.service
	chmod 0644 $(DESTDIR)$(SYSTEMD_USER_UNIT_DIR)/kait2en-t2-autobrightness.service
	install -d $(DESTDIR)$(DATADIR)/gnome-shell/extensions/$(EXTENSION_UUID)
	install -m 0644 integration/gnome-shell/$(EXTENSION_UUID)/extension.js integration/gnome-shell/$(EXTENSION_UUID)/metadata.json $(DESTDIR)$(DATADIR)/gnome-shell/extensions/$(EXTENSION_UUID)/
uninstall:
	rm -f $(DESTDIR)$(BINDIR)/t2-autobrightness
	rm -f $(DESTDIR)$(SYSTEMD_USER_UNIT_DIR)/kait2en-t2-autobrightness.service
	rm -rf $(DESTDIR)$(DATADIR)/gnome-shell/extensions/$(EXTENSION_UUID)
clean:
	$(CARGO) clean --target-dir $(TARGET_DIR)
