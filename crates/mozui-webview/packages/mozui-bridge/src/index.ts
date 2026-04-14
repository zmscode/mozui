import type { MozuiError, UnlistenFn } from "./types.js";

export type { MozuiError, UnlistenFn } from "./types.js";

/** Returns true if running inside a mozui webview with the bridge injected. */
export function isBridge(): boolean {
	return typeof window !== "undefined" && typeof window.__mozui_dispatch === "function";
}

/**
 * Invoke a registered Rust command.
 *
 * Outside a mozui webview, rejects with an error.
 */
export function invoke<T = unknown>(command: string, args?: unknown): Promise<T> {
	if (!isBridge()) {
		return Promise.reject(new Error("[mozui] invoke() called outside a mozui webview"));
	}
	return window.mozui.invoke<T>(command, args);
}

/**
 * Listen for events pushed from Rust via `emit_to_js`.
 *
 * Outside a mozui webview, returns a no-op unlisten function.
 */
export function listen<T = unknown>(event: string, handler: (payload: T) => void): UnlistenFn {
	if (!isBridge()) return () => {};
	return window.mozui.listen<T>(event, handler);
}

/**
 * Emit an event from JS to Rust.
 *
 * Outside a mozui webview, silently does nothing.
 */
export function emit(event: string, payload?: unknown): void {
	if (!isBridge()) return;
	window.mozui.emit(event, payload);
}
