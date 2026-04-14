// Type declarations for the Silk renderer bridge (injected by silk-runtime)

interface SilkFs {
	readText(path: string): Promise<string>;
	writeText(path: string, contents: string): Promise<boolean>;
	exists(path: string): Promise<boolean>;
	mkdir(path: string, options?: { recursive?: boolean }): Promise<boolean>;
	remove(path: string): Promise<boolean>;
}

interface SilkClipboard {
	read(): Promise<string>;
	write(text: string): Promise<boolean>;
}

interface DialogFilter {
	extensions: string[];
}

interface SilkDialog {
	open(options?: { title?: string; filters?: DialogFilter[]; multiple?: boolean }): Promise<string | string[] | null>;
	save(options?: { title?: string; defaultPath?: string }): Promise<string | null>;
	message(message: string, options?: { title?: string; type?: "info" | "warning" | "error" }): Promise<boolean>;
}

interface SilkShell {
	open(url: string): Promise<boolean>;
}

interface SilkRenderer {
	invoke<T = unknown>(command: string, args?: unknown): Promise<T>;
	listen(event: string, handler: (payload: unknown) => void): void;
	emit(event: string, payload?: unknown): void;
	fs: SilkFs;
	clipboard: SilkClipboard;
	dialog: SilkDialog;
	shell: SilkShell;
}

declare const Silk: SilkRenderer;

// Main process types (for main.ts)

interface WindowOptions {
	url?: string;
	title?: string;
	width?: number;
	height?: number;
	minWidth?: number;
	minHeight?: number;
	resizable?: boolean;
	/** "default" = standard titlebar, "hidden" = transparent titlebar, "hiddenInset" = transparent with inset traffic lights */
	titlebarStyle?: "default" | "hidden" | "hiddenInset";
	/** Position of macOS traffic light buttons (used with titlebarStyle: "hiddenInset") */
	trafficLightPosition?: { x: number; y: number };
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
