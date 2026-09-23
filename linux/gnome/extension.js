// SPDX-License-Identifier: MIT
import Clutter from 'gi://Clutter';
import Cogl from 'gi://Cogl';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GdkPixbuf from 'gi://GdkPixbuf';
import Soup from 'gi://Soup?version=3.0';
import St from 'gi://St';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as Config from 'resource:///org/gnome/shell/misc/config.js';

export default class AsijiWallpaper extends Extension {
    enable() {
        this._enabled = true;
        this._actors = [];
        this._url = '';
        this._busy = false;
        this._lastFrame = 0;
        this._fps = 30;
        this._image = null;
        this._imageSize = '';
        this._error = '';
        this._directory = GLib.build_filenamev([GLib.get_user_runtime_dir(), 'asiji-wallpaper']);
        GLib.mkdir_with_parents(this._directory, 0o700);
        this._session = new Soup.Session({timeout: 2});
        this._cancellable = new Gio.Cancellable();
        this._monitorId = Main.layoutManager.connect('monitors-changed', () => this._clear());
        this._heartbeat = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 500, () => {
            GLib.file_set_contents(this._directory + '/gnome-ready', 'ready');
            try {
                const [, data] = GLib.file_get_contents(this._directory + '/gnome.json');
                if (data.length > 4096)
                    throw new Error('Invalid ASIJI state');
                const state = JSON.parse(new TextDecoder().decode(data));
                if (!/^http:\/\/127\.0\.0\.1:[0-9]+\/[a-f0-9]{64}$/.test(state.url))
                    throw new Error('Invalid ASIJI frame URL');
                if (this._url !== state.url) {
                    this._clear();
                    this._url = state.url;
                }
                this._fps = Math.max(10, Math.min(60, Number(state.fps) || 30));
            } catch {
                this._url = '';
                this._clear();
            }
            return GLib.SOURCE_CONTINUE;
        });
        this._timer = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 16, () => {
            const now = GLib.get_monotonic_time();
            if (Main.sessionMode.isLocked) {
                this._clear();
            } else if (this._url && !this._busy && now - this._lastFrame >= 1000000 / this._fps) {
                this._lastFrame = now;
                this._request();
            }
            return GLib.SOURCE_CONTINUE;
        });
    }

    _clear() {
        for (const actor of this._actors)
            actor.destroy();
        this._actors = [];
    }

    _request() {
        this._busy = true;
        const url = this._url;
        const message = Soup.Message.new('GET', url + '/frame.bmp');
        this._session.send_and_read_async(message, GLib.PRIORITY_DEFAULT, this._cancellable, (session, result) => {
            try {
                const bytes = session.send_and_read_finish(result);
                if (!this._enabled || this._url !== url || Main.sessionMode.isLocked)
                    return;
                if (message.status_code !== 200 || bytes.get_size() > 160000000)
                    throw new Error('ASIJI frame unavailable');
                const loader = GdkPixbuf.PixbufLoader.new_with_type('bmp');
                loader.write(bytes.get_data());
                loader.close();
                const pixbuf = loader.get_pixbuf();
                const size = pixbuf.width + 'x' + pixbuf.height;
                if (this._imageSize !== size) {
                    this._image = St.ImageContent.new_with_preferred_size(pixbuf.width, pixbuf.height);
                    this._imageSize = size;
                }
                const data = [pixbuf.get_pixels(), Cogl.PixelFormat.RGB_888, pixbuf.width, pixbuf.height, pixbuf.rowstride];
                if (Number.parseInt(Config.PACKAGE_VERSION) >= 48)
                    data.unshift(global.stage.context.get_backend().get_cogl_context());
                this._image.set_data(...data);
                if (this._actors.length === 0) {
                    for (const monitor of Main.layoutManager.monitors) {
                        const actor = new St.Widget({reactive: false, style: 'background-color: black;'});
                        actor.set_position(monitor.x, monitor.y);
                        actor.set_size(monitor.width, monitor.height);
                        actor.content_gravity = Clutter.ContentGravity.RESIZE_ASPECT;
                        Main.layoutManager._backgroundGroup.add_child(actor);
                        this._actors.push(actor);
                    }
                }
                for (const actor of this._actors)
                    actor.content = this._image;
                this._error = '';
            } catch (error) {
                if (this._enabled) {
                    this._clear();
                    if (this._error !== error.message) {
                        console.error('ASIJI wallpaper: ' + error.message);
                        this._error = error.message;
                    }
                }
            } finally {
                this._busy = false;
            }
        });
    }

    disable() {
        this._enabled = false;
        for (const source of [this._heartbeat, this._timer]) {
            if (source)
                GLib.source_remove(source);
        }
        this._cancellable?.cancel();
        this._session?.abort();
        if (this._monitorId)
            Main.layoutManager.disconnect(this._monitorId);
        this._clear();
        GLib.unlink(this._directory + '/gnome-ready');
        this._session = null;
        this._cancellable = null;
        this._heartbeat = this._timer = this._monitorId = 0;
        this._url = '';
        this._image = null;
        this._imageSize = '';
    }
}
