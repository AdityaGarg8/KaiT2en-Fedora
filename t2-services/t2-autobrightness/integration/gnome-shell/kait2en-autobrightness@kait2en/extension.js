// SPDX-License-Identifier: GPL-3.0-or-later

import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {Extension, InjectionManager} from 'resource:///org/gnome/shell/extensions/extension.js';

// gnome-shell takes a backlight change it did not make into the monitor's
// scale only. The global scale behind the slider stays where it was, so the
// slider shows a stale value and a touch jumps the panel back to it.
// After every sync the global scale is set to the brightest monitor again,
// the way the shell does it after a user change.
function followBacklight(manager) {
    const global = manager.globalScale;
    const scales = [...manager.scales];
    if (!global || scales.length === 0 || manager._inhibitUpdates)
        return;

    const max = Math.max(...scales.map(scale => scale.value));
    if (Math.abs(global.value - max) < 0.001)
        return;

    // Inhibited, the value handlers return early: no user-update, so no
    // BrightnessChanged on D-Bus, and t2-autobrightness sees no manual change.
    manager._inhibitUpdates = true;
    try {
        if (max > 0.01)
            scales.forEach(scale => scale.updateScaleFactor(max));
        global.value = max;
    } finally {
        manager._inhibitUpdates = false;
    }
}

export default class AutoBrightnessExtension extends Extension {
    enable() {
        const manager = Main.brightnessManager;
        if (!manager)
            return;

        this._injections = new InjectionManager();
        this._injections.overrideMethod(manager, '_sync', original => function (...args) {
            original.call(this, ...args);
            followBacklight(this);
        });
        followBacklight(manager);
    }

    disable() {
        this._injections?.clear();
        this._injections = null;
    }
}
