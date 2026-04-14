// mozui IPC bridge — injected via with_initialization_script before any page code runs.
// Closure-scoped internals are not reachable from page code.
(function () {
	"use strict";

	var _pending = new Map();
	var _listeners = new Map();

	function _send(json) {
		if (window.ipc) {
			window.ipc.postMessage(json);
		} else if (window.chrome && window.chrome.webview) {
			window.chrome.webview.postMessage(json);
		}
	}

	// Called by Rust via evaluate_script("window.__mozui_dispatch('...')")
	Object.defineProperty(window, "__mozui_dispatch", {
		value: function (rawJson) {
			var msg;
			try {
				msg = JSON.parse(rawJson);
			} catch (e) {
				return;
			}
			if (!msg || !msg.__mozui) return;

			if (msg.type === "response") {
				var pending = _pending.get(msg.id);
				if (!pending) return;
				_pending.delete(msg.id);
				if (msg.ok) {
					pending.resolve(msg.payload);
				} else {
					var err = new Error(msg.error && msg.error.message ? msg.error.message : "IPC error");
					err.code = msg.error && msg.error.code ? msg.error.code : "INTERNAL";
					pending.reject(err);
				}
			} else if (msg.type === "event") {
				var handlers = _listeners.get(msg.name);
				if (handlers) {
					handlers.forEach(function (h) {
						h(msg.payload);
					});
				}
			}
		},
		writable: false,
		configurable: false,
		enumerable: false,
	});

	var _mozui = {
		invoke: function (command, args) {
			return new Promise(function (resolve, reject) {
				var id = crypto.randomUUID ? crypto.randomUUID() : Math.random().toString(36).slice(2) + Date.now().toString(36);
				_pending.set(id, { resolve: resolve, reject: reject });
				_send(
					JSON.stringify({
						__mozui: true,
						id: id,
						type: "invoke",
						command: command,
						args: args !== undefined ? args : null,
					}),
				);
				setTimeout(function () {
					if (_pending.has(id)) {
						_pending.delete(id);
						// Notify Rust to cancel the in-flight task
						_send(
							JSON.stringify({
								__mozui: true,
								id: id,
								type: "cancel",
							}),
						);
						var err = new Error("mozui invoke timed out: " + command);
						err.code = "TIMEOUT";
						reject(err);
					}
				}, 30000);
			});
		},

		listen: function (event, handler) {
			if (!_listeners.has(event)) _listeners.set(event, new Set());
			_listeners.get(event).add(handler);
			return function () {
				_listeners.get(event) && _listeners.get(event).delete(handler);
			};
		},

		emit: function (event, payload) {
			_send(
				JSON.stringify({
					__mozui: true,
					type: "emit",
					name: event,
					payload: payload !== undefined ? payload : null,
				}),
			);
		},
	};

	Object.freeze(_mozui);
	Object.defineProperty(window, "mozui", {
		value: _mozui,
		writable: false,
		configurable: false,
		enumerable: true,
	});
})();
