// Keeps the idlefx companion window above other windows, on every workspace,
// at the bottom-right corner of the work area. The window is recognised by its
// Wayland app id, which GNOME Shell reports as the WM class.
//
// It also tells the dog when you leave and come back: `idlefx-companion poke
// sleep` when the screen locks or after 5 minutes without input, `poke wake`
// when it unlocks or input returns.

import GLib from 'gi://GLib';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

const APP_ID = 'idlefx-companion';
const MARGIN = 16;
const IDLE_MS = 5 * 60 * 1000;

export default class IdlefxCompanionExtension extends Extension {
    enable() {
        this._created = global.display.connect('window-created', (_display, win) => this._watch(win));
        for (const actor of global.get_window_actors())
            this._place(actor.meta_window);

        this._away = false;
        this._session = Main.sessionMode.connect('updated', () => this._lockChanged());
        this._idle = global.backend.get_core_idle_monitor();
        this._idleWatch = this._idle.add_idle_watch(IDLE_MS, () => this._leave());
        this._activeWatch = 0;
    }

    disable() {
        if (this._created) {
            global.display.disconnect(this._created);
            this._created = null;
        }
        if (this._session) {
            Main.sessionMode.disconnect(this._session);
            this._session = null;
        }
        if (this._idle) {
            for (const id of [this._idleWatch, this._activeWatch]) {
                if (id)
                    this._idle.remove_watch(id);
            }
            this._idle = null;
            this._idleWatch = this._activeWatch = 0;
        }
    }

    _lockChanged() {
        if (Main.sessionMode.isLocked)
            this._leave();
        else
            this._back();
    }

    // You left: the screen locked, or no input for a while.
    _leave() {
        if (this._away)
            return;
        this._away = true;
        this._poke('sleep');
        // One-shot: fires at the next input.
        this._activeWatch = this._idle.add_user_active_watch(() => {
            this._activeWatch = 0;
            if (!Main.sessionMode.isLocked)
                this._back();
        });
    }

    _back() {
        if (!this._away)
            return;
        this._away = false;
        this._poke('wake');
    }

    // The session's PATH often lacks ~/.cargo/bin, so look there too.
    _poke(event) {
        const bin = GLib.find_program_in_path(APP_ID) ??
            GLib.build_filenamev([GLib.get_home_dir(), '.cargo', 'bin', APP_ID]);
        try {
            GLib.spawn_async(null, [bin, 'poke', event], null, GLib.SpawnFlags.DEFAULT, null);
        } catch (_e) {
            // No dog installed: nothing to tell.
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
