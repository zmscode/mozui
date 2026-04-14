import type { MozuiBridge } from "./types.js";

interface MockBridgeHandle {
	/** Push a mock event to all registered listeners. */
	push: (event: string, payload: unknown) => void;
}

/**
 * Install a mock mozui bridge on `window.mozui` for testing renderer
 * components without a running Rust backend.
 *
 * @param handlers - Map of command names to mock handler functions.
 * @returns A handle to push mock events into listeners.
 */
export function installMockBridge(handlers: Record<string, (args: unknown) => unknown> = {}): MockBridgeHandle {
	const listeners = new Map<string, Set<(payload: unknown) => void>>();

	const bridge: MozuiBridge = {
		invoke: async <T = unknown>(command: string, args?: unknown): Promise<T> => {
			const handler = handlers[command];
			if (!handler) {
				const err = new Error(`No mock for: ${command}`) as Error & {
					code: string;
				};
				err.code = "UNKNOWN_COMMAND";
				throw err;
			}
			return handler(args) as T;
		},

		listen: <T = unknown>(event: string, handler: (payload: T) => void): (() => void) => {
			if (!listeners.has(event)) listeners.set(event, new Set());
			listeners.get(event)!.add(handler as (payload: unknown) => void);
			return () => listeners.get(event)?.delete(handler as (payload: unknown) => void);
		},

		emit: () => {},
	};

	Object.defineProperty(window, "mozui", {
		value: bridge,
		writable: true, // writable so tests can reinstall
		configurable: true,
		enumerable: true,
	});

	Object.defineProperty(window, "__mozui_dispatch", {
		value: () => {},
		writable: true,
		configurable: true,
	});

	return {
		push: (event: string, payload: unknown) => {
			listeners.get(event)?.forEach((h) => h(payload));
		},
	};
}
