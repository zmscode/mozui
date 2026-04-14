export interface MozuiError extends Error {
	code: string;
}

export type UnlistenFn = () => void;

export interface MozuiBridge {
	invoke<T = unknown>(command: string, args?: unknown): Promise<T>;
	listen<T = unknown>(event: string, handler: (payload: T) => void): UnlistenFn;
	emit(event: string, payload?: unknown): void;
}

declare global {
	interface Window {
		readonly mozui: MozuiBridge;
		readonly __mozui_dispatch: (rawJson: string) => void;
		readonly __mozui_origin: string;
		readonly __mozui_scheme: string;
	}
}
