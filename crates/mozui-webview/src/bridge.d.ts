// mozui bridge TypeScript declarations
// These types describe the `window.mozui` global injected by the mozui-webview bridge.

export interface MozuiError extends Error {
  code: string;
}

export type UnlistenFn = () => void;

export interface MozuiBridge {
  /**
   * Invoke a registered Rust command.
   * Returns a promise that resolves with the command's return value,
   * or rejects with a `MozuiError` containing an error code.
   *
   * Times out after 30 seconds with code `TIMEOUT`.
   */
  invoke<T = unknown>(command: string, args?: unknown): Promise<T>;

  /**
   * Listen for events pushed from Rust via `emit_to_js`.
   * Returns an unlisten function to remove the handler.
   */
  listen<T = unknown>(event: string, handler: (payload: T) => void): UnlistenFn;

  /**
   * Emit an event from JS to Rust.
   * Subject to the `js_emit_allowlist` in Capabilities.
   */
  emit(event: string, payload?: unknown): void;
}

declare global {
  interface Window {
    /** mozui IPC bridge — frozen, non-writable, non-configurable. */
    readonly mozui: MozuiBridge;

    /** @internal Dispatch function called by Rust via evaluate_script. */
    readonly __mozui_dispatch: (rawJson: string) => void;

    /** @internal Platform origin, e.g. "mozui://localhost" or "https://mozui.localhost". */
    readonly __mozui_origin: string;

    /** @internal Platform scheme, e.g. "mozui" or "https". */
    readonly __mozui_scheme: string;
  }
}

export {};
