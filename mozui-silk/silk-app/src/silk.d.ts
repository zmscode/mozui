interface WindowOptions {
  url?: string;
  title?: string;
  width?: number;
  height?: number;
  minWidth?: number;
  minHeight?: number;
  resizable?: boolean;
}

interface SilkMain {
  onReady(callback: () => void | Promise<void>): void;
  createWindow(label: string, options?: WindowOptions): Promise<{ label: string }>;
  closeWindow(label: string): Promise<boolean>;
  setTitle(label: string, title: string): Promise<boolean>;
  emitTo(label: string, event: string, payload?: unknown): Promise<boolean>;
  emitAll(event: string, payload?: unknown): Promise<boolean>;
  handle<TArgs = unknown, TResult = unknown>(
    command: string,
    handler: (args: TArgs) => TResult | Promise<TResult>,
  ): void;
  quit(): Promise<boolean>;
}

declare const Silk: SilkMain;
