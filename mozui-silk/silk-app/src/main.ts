// Silk Todo — main process
// All app logic lives here. Renderers invoke commands via Silk.invoke().

interface Todo {
	id: string;
	text: string;
	done: boolean;
}

function uid(): string {
	return Math.random().toString(36).slice(2) + Date.now().toString(36);
}

Silk.onReady(() => {
	let todos: Todo[] = [];

	Silk.createWindow("main", {
		url: "./index.html",
		title: "Silk Todo",
		width: 460,
		height: 680,
		minWidth: 360,
		minHeight: 480,
	});

	Silk.handle("list-todos", () => todos);

	Silk.handle<{ text: string }>("add-todo", (args) => {
		const text = args.text?.trim();
		if (!text) return null;

		const todo: Todo = {
			id: uid(),
			text,
			done: false,
		};
		todos.push(todo);
		Silk.emitAll("todos:changed", todos);
		return todo;
	});

	Silk.handle<{ id: string }>("toggle-todo", (args) => {
		const todo = todos.find((t) => t.id === args.id);
		if (!todo) return null;
		todo.done = !todo.done;
		Silk.emitAll("todos:changed", todos);
		return todo;
	});

	Silk.handle<{ id: string }>("delete-todo", (args) => {
		todos = todos.filter((t) => t.id !== args.id);
		Silk.emitAll("todos:changed", todos);
		return true;
	});

	Silk.handle<{ id: string; text: string }>("edit-todo", (args) => {
		const newText = args.text?.trim();
		if (!newText) return null;
		const todo = todos.find((t) => t.id === args.id);
		if (!todo) return null;
		todo.text = newText;
		Silk.emitAll("todos:changed", todos);
		return todo;
	});

	Silk.handle("clear-completed", () => {
		todos = todos.filter((t) => !t.done);
		Silk.emitAll("todos:changed", todos);
		return true;
	});
});
