// Silk Todo — renderer process
// UI logic for the todo app. Communicates with main process via Silk.invoke().

interface Todo {
	id: string;
	text: string;
	done: boolean;
}

let todos: Todo[] = [];
let filter: "all" | "active" | "done" = "all";
let editingId: string | null = null;

const inputEl = document.getElementById("input") as HTMLInputElement;
const listEl = document.getElementById("list")!;
const addBtn = document.getElementById("add-btn")!;

// --- Event wiring (no inline handlers) ---

inputEl.addEventListener("keydown", (e) => {
	if (e.key === "Enter") addTodo();
});

addBtn.addEventListener("click", () => addTodo());

// Filter buttons
document.querySelector(".filter-bar")!.addEventListener("click", (e) => {
	const btn = (e.target as HTMLElement).closest(".filter-btn") as HTMLElement | null;
	if (!btn) return;
	const f = btn.dataset.filter as "all" | "active" | "done";
	if (f) setFilter(f, btn);
});

// Event delegation on list for check, delete, edit, clear
listEl.addEventListener("click", (e) => {
	const target = e.target as HTMLElement;

	// Toggle checkbox
	const check = target.closest(".check") as HTMLElement | null;
	if (check) {
		const id = check.dataset.id;
		if (id) toggleTodo(id);
		return;
	}

	// Delete button
	const del = target.closest(".delete-btn") as HTMLElement | null;
	if (del) {
		const id = del.dataset.id;
		if (id) deleteTodo(id);
		return;
	}

	// Clear completed
	if (target.closest(".clear-btn")) {
		clearCompleted();
		return;
	}
});

listEl.addEventListener("dblclick", (e) => {
	const text = (e.target as HTMLElement).closest(".todo-text") as HTMLElement | null;
	if (text) {
		const id = text.dataset.id;
		if (id) startEdit(id);
	}
});

// Listen for real-time updates from main process
Silk.listen("todos:changed", (updated: Todo[]) => {
	todos = updated;
	render();
});

// Load initial data
loadTodos();

// --- Functions ---

async function loadTodos() {
	try {
		todos = await Silk.invoke("list-todos");
		render();
	} catch (e) {
		console.error("Failed to load todos:", e);
	}
}

async function addTodo() {
	const text = inputEl.value.trim();
	if (!text) return;
	try {
		await Silk.invoke("add-todo", { text });
		inputEl.value = "";
	} catch (e) {
		console.error("Failed to add todo:", e);
	}
}

async function toggleTodo(id: string) {
	try {
		await Silk.invoke("toggle-todo", { id });
	} catch (e) {
		console.error("Failed to toggle todo:", e);
	}
}

async function deleteTodo(id: string) {
	try {
		await Silk.invoke("delete-todo", { id });
	} catch (e) {
		console.error("Failed to delete todo:", e);
	}
}

function startEdit(id: string) {
	editingId = id;
	render();
	const el = document.getElementById("edit-" + id) as HTMLInputElement | null;
	if (el) {
		el.focus();
		el.select();
	}
}

async function commitEdit(id: string) {
	const el = document.getElementById("edit-" + id) as HTMLInputElement | null;
	if (!el) return;
	const newText = el.value.trim();
	editingId = null;
	if (!newText) {
		render();
		return;
	}
	try {
		await Silk.invoke("edit-todo", { id, text: newText });
	} catch (e) {
		console.error("Failed to edit todo:", e);
		render();
	}
}

async function clearCompleted() {
	try {
		await Silk.invoke("clear-completed");
	} catch (e) {
		console.error("Failed to clear completed:", e);
	}
}

function setFilter(f: "all" | "active" | "done", btn: HTMLElement) {
	filter = f;
	document.querySelectorAll(".filter-btn").forEach((b) => {
		b.classList.remove("active");
	});
	btn.classList.add("active");
	render();
}

function render() {
	const total = todos.length;
	const doneCount = todos.filter((t) => t.done).length;
	document.getElementById("total")!.textContent = String(total);
	document.getElementById("remaining")!.textContent = String(total - doneCount);
	document.getElementById("completed")!.textContent = String(doneCount);

	const visible = todos.filter((t) => {
		if (filter === "active") return !t.done;
		if (filter === "done") return t.done;
		return true;
	});

	if (visible.length === 0) {
		const msg = total === 0 ? "No todos yet" : filter === "done" ? "Nothing completed" : "All done!";
		listEl.innerHTML = '<div class="empty"><div class="icon"></div>' + msg + "</div>";
		return;
	}

	let html = "";
	for (const t of visible) {
		const cls = "todo" + (t.done ? " done" : "");
		const check =
			'<svg width="12" height="12" viewBox="0 0 12 12" fill="none"><path d="M2.5 6L5 8.5L9.5 3.5" stroke="white" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>';
		let textContent: string;
		if (editingId === t.id) {
			textContent = '<input id="edit-' + t.id + '" class="edit-input" value="' + escapeHtml(t.text) + '" data-id="' + t.id + '">';
		} else {
			textContent = escapeHtml(t.text);
		}

		html +=
			'<div class="' +
			cls +
			'">' +
			'<div class="check" data-id="' +
			t.id +
			'">' +
			check +
			"</div>" +
			'<div class="todo-text" data-id="' +
			t.id +
			'">' +
			textContent +
			"</div>" +
			'<button class="delete-btn" data-id="' +
			t.id +
			'">&times;</button>' +
			"</div>";
	}

	if (doneCount > 0) {
		html += '<button class="clear-btn">Clear ' + doneCount + " completed</button>";
	}

	listEl.innerHTML = html;

	// Wire up edit input events after innerHTML
	if (editingId) {
		const el = document.getElementById("edit-" + editingId) as HTMLInputElement | null;
		if (el) {
			el.focus();
			el.select();
			el.addEventListener("blur", () => commitEdit(el.dataset.id!));
			el.addEventListener("keydown", (e) => {
				if (e.key === "Enter") commitEdit(el.dataset.id!);
				if (e.key === "Escape") {
					editingId = null;
					render();
				}
			});
		}
	}
}

function escapeHtml(s: string): string {
	return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}
