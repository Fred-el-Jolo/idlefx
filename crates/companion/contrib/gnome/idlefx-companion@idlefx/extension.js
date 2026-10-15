// Keeps the idlefx companion window above other windows, on every workspace,
// at the bottom-right corner of the work area. The window is recognised by its
// Wayland app id, which GNOME Shell reports as the WM class.

import GLib from 'gi://GLib';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

const APP_ID = 'idlefx-companion';
const MARGIN = 16;

export default class IdlefxCompanionExtension extends Extension {
    enable() {
        this._created = global.display.connect('window-created', (_display, win) => this._watch(win));
        for (const actor of global.get_window_actors())
            this._place(actor.meta_window);
    }

    disable() {
        if (this._created) {
            global.display.disconnect(this._created);
            this._created = null;
        }
    }

    // The app id and the size are known once the window shows its first frame.
    _watch(win) {
        const actor = win.get_compositor_private();
        if (actor) {
            const id = actor.connect('first-frame', () => {
                actor.disconnect(id);
                this._place(win);
            });
        } else {
            GLib.idle_add(GLib.PRIORITY_DEFAULT, () => {
                this._place(win);
                return GLib.SOURCE_REMOVE;
            });
        }
    }

    _place(win) {
        if (!win || win.get_wm_class() !== APP_ID)
            return;
        win.make_above();
        win.stick();
        const area = win.get_work_area_current_monitor();
        const frame = win.get_frame_rect();
        win.move_frame(
            true,
            area.x + area.width - frame.width - MARGIN,
            area.y + area.height - frame.height - MARGIN);
    }
}
