// Silk renderer bridge — injected into every renderer webview.
// Provides the Silk global for invoking commands and listening for events.
// Uses mozui.emit/listen for the __silk:invoke round-trip (not mozui.invoke)
// because user-defined commands are async through the main process.
(function () {
	"use strict";

	var _pending = new Map();

	// Listen for responses routed back from Rust.
	window.mozui.listen("__silk:invoke-response", function (msg) {
		var p = _pending.get(msg.invokeId);
		if (!p) return;
		_pending.delete(msg.invokeId);
		if (msg.ok) {
			p.resolve(msg.payload);
		} else {
			var err = new Error(msg.error && msg.error.message ? msg.error.message : "invoke error");
			err.code = msg.error && msg.error.code ? msg.error.code : "INTERNAL";
			p.reject(err);
		}
	});

	var Silk = {
		invoke: function (command, args) {
			return new Promise(function (resolve, reject) {
				var id = crypto.randomUUID ? crypto.randomUUID() : Math.random().toString(36).slice(2) + Date.now().toString(36);
				_pending.set(id, { resolve: resolve, reject: reject });

				// Send via emit — Rust intercepts JsEmitEvent and routes to main process.
				window.mozui.emit("__silk:invoke", {
					invokeId: id,
					command: command,
					args: args !== undefined ? args : null,
				});

				// Timeout
				setTimeout(function () {
					if (_pending.has(id)) {
						_pending.delete(id);
						var err = new Error("Silk invoke timed out: " + command);
						err.code = "TIMEOUT";
						reject(err);
					}
				}, 30000);
			});
		},

		listen: function (event, handler) {
			return window.mozui.listen(event, handler);
		},

		emit: function (event, payload) {
			window.mozui.emit(event, payload);
		},

		// Built-in commands — handled directly in Rust, no main process round-trip.
		fs: {
			readText: function (path) {
				return window.mozui.invoke("__silk:fs-read-text", { path: path });
			},
			writeText: function (path, contents) {
				return window.mozui.invoke("__silk:fs-write-text", { path: path, contents: contents });
			},
			exists: function (path) {
				return window.mozui.invoke("__silk:fs-exists", { path: path });
			},
			mkdir: function (path, options) {
				return window.mozui.invoke("__silk:fs-mkdir", { path: path, recursive: options && options.recursive });
			},
			remove: function (path) {
				return window.mozui.invoke("__silk:fs-remove", { path: path });
			},
		},

		clipboard: {
			read: function () {
				return window.mozui.invoke("__silk:clipboard-read", {});
			},
			write: function (text) {
				return window.mozui.invoke("__silk:clipboard-write", { text: text });
			},
		},

		dialog: {
			open: function (options) {
				return window.mozui.invoke("__silk:dialog-open", options || {});
			},
			save: function (options) {
				return window.mozui.invoke("__silk:dialog-save", options || {});
			},
			message: function (message, options) {
				return window.mozui.invoke("__silk:dialog-message", Object.assign({ message: message }, options || {}));
			},
		},

		shell: {
			open: function (url) {
				return window.mozui.invoke("__silk:shell-open", { url: url });
			},
		},
	};

	Object.freeze(Silk.fs);
	Object.freeze(Silk.clipboard);
	Object.freeze(Silk.dialog);
	Object.freeze(Silk.shell);
	Object.freeze(Silk);
	Object.defineProperty(window, "Silk", {
		value: Silk,
		writable: false,
		configurable: false,
		enumerable: true,
	});
})();
