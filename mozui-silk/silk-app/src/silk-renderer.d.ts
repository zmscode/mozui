// Type declarations for the Silk renderer bridge (injected by silk-runtime)

interface SilkRenderer {
  invoke<T = unknown>(command: string, args?: unknown): Promise<T>;
  listen(event: string, handler: (payload: unknown) => void): void;
  emit(event: string, payload?: unknown): void;
}

declare const Silk: SilkRenderer;
