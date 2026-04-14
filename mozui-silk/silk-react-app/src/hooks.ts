import { useState, useEffect, useCallback, useRef } from "react";

export interface Todo {
	id: string;
	text: string;
	done: boolean;
}

export type Filter = "all" | "active" | "done";

const STORAGE_PATH = "todos.json";

function uid(): string {
	return Math.random().toString(36).slice(2) + Date.now().toString(36);
}

async function loadFromDisk(): Promise<Todo[]> {
	try {
		const exists = await Silk.fs.exists(STORAGE_PATH);
		if (!exists) return [];
		const json = await Silk.fs.readText(STORAGE_PATH);
		return JSON.parse(json);
	} catch {
		return [];
	}
}

async function saveToDisk(todos: Todo[]) {
	try {
		await Silk.fs.writeText(STORAGE_PATH, JSON.stringify(todos));
	} catch (e) {
		console.error("Failed to save todos:", e);
	}
}

export function useTodos() {
	const [todos, setTodos] = useState<Todo[]>([]);
	const todosRef = useRef(todos);
	todosRef.current = todos;

	useEffect(() => {
		loadFromDisk().then(setTodos);
	}, []);

	const update = useCallback((fn: (prev: Todo[]) => Todo[]) => {
		setTodos((prev) => {
			const next = fn(prev);
			saveToDisk(next);
			return next;
		});
	}, []);

	const addTodo = useCallback(
		(text: string) => {
			update((prev) => [...prev, { id: uid(), text, done: false }]);
		},
		[update],
	);

	const toggleTodo = useCallback(
		(id: string) => {
			update((prev) => prev.map((t) => (t.id === id ? { ...t, done: !t.done } : t)));
		},
		[update],
	);

	const deleteTodo = useCallback(
		(id: string) => {
			update((prev) => prev.filter((t) => t.id !== id));
		},
		[update],
	);

	const editTodo = useCallback(
		(id: string, text: string) => {
			update((prev) => prev.map((t) => (t.id === id ? { ...t, text } : t)));
		},
		[update],
	);

	const clearCompleted = useCallback(() => {
		update((prev) => prev.filter((t) => !t.done));
	}, [update]);

	const exportTodos = useCallback(async () => {
		const path = await Silk.dialog.save({
			title: "Export Todos",
			defaultPath: "todos.json",
		});
		if (path) {
			await Silk.fs.writeText(path as string, JSON.stringify(todosRef.current, null, 2));
			await Silk.dialog.message(`Exported ${todosRef.current.length} todos.`);
		}
	}, []);

	const importTodos = useCallback(async () => {
		const path = await Silk.dialog.open({
			title: "Import Todos",
			filters: [{ extensions: ["json"] }],
		});
		if (path) {
			try {
				const json = await Silk.fs.readText(path as string);
				const imported = JSON.parse(json) as Todo[];
				update((prev) => [
					...prev,
					...imported.map((t) => ({ id: uid(), text: t.text, done: t.done })),
				]);
			} catch (e) {
				await Silk.dialog.message("Failed to import: invalid JSON file.", { type: "error" });
			}
		}
	}, [update]);

	return { todos, addTodo, toggleTodo, deleteTodo, editTodo, clearCompleted, exportTodos, importTodos };
}
