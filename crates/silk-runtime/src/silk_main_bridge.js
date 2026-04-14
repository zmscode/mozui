// Silk main process bridge — injected into the hidden main webview.
// Provides the Silk global for registering command handlers, creating windows, etc.
// Layered on top of the mozui bridge (window.mozui must already exist).
(function () {
	"use strict";

	var _handlers = new Map();

	// Listen for forwarded invokes from Rust (sent via emit_to_js).
	window.mozui.listen("__silk:forward", function (msg) {
		var handler = _handlers.get(msg.command);
		if (!handler) {
			window.mozui.emit("__silk:response", {
				correlationId: msg.correlationId,
				ok: false,
				error: { code: "UNKNOWN_COMMAND", message: "no handler for: " + msg.command },
			});
			return;
		}

		Promise.resolve()
			.then(function () {
				return handler(msg.args);
			})
			.then(function (result) {
				window.mozui.emit("__silk:response", {
					correlationId: msg.correlationId,
					ok: true,
					payload: result !== undefined ? result : null,
				});
			})
			.catch(function (err) {
				window.mozui.emit("__silk:response", {
					correlationId: msg.correlationId,
					ok: false,
					error: {
						code: "HANDLER_ERROR",
						message: err && err.message ? err.message : String(err),
					},
				});
			});
	});

	var Silk = {
		// Register a command handler callable from renderer windows.
		handle: function (command, handler) {
			_handlers.set(command, handler);
		},

		// Create a new renderer window.
		createWindow: function (label, options) {
			return window.mozui.invoke("__silk:create-window", {
				label: label,
				url: options && options.url ? options.url : undefined,
				title: options && options.title ? options.title : undefined,
				width: options && options.width ? options.width : undefined,
				height: options && options.height ? options.height : undefined,
				minWidth: options && options.minWidth ? options.minWidth : undefined,
				minHeight: options && options.minHeight ? options.minHeight : undefined,
				resizable: options && options.resizable !== undefined ? options.resizable : undefined,
				titlebarStyle: options && options.titlebarStyle ? options.titlebarStyle : undefined,
				trafficLightPosition: options && options.trafficLightPosition ? options.trafficLightPosition : undefined,
			});
		},

		// Close a renderer window by label.
		closeWindow: function (label) {
			return window.mozui.invoke("__silk:close-window", { label: label });
		},

		// Set a window's title.
		setTitle: function (label, title) {
			return window.mozui.invoke("__silk:set-title", { label: label, title: title });
		},

		// Emit an event to a specific renderer window.
		emitTo: function (label, event, payload) {
			return window.mozui.invoke("__silk:emit-to", {
				label: label,
				event: event,
				payload: payload !== undefined ? payload : null,
			});
		},

		// Emit an event to all renderer windows.
		emitAll: function (event, payload) {
			return window.mozui.invoke("__silk:emit-all", {
				event: event,
				payload: payload !== undefined ? payload : null,
			});
		},

		// Quit the application.
		quit: function () {
			return window.mozui.invoke("__silk:quit", {});
		},

		// Run callback when the main process is ready.
		onReady: function (callback) {
			if (document.readyState === "complete") {
				setTimeout(callback, 0);
			} else {
				window.addEventListener("load", function () {
					setTimeout(callback, 0);
				});
			}
		},
	};

	Object.freeze(Silk);
	Object.defineProperty(window, "Silk", {
		value: Silk,
		writable: false,
		configurable: false,
		enumerable: true,
	});
})();
