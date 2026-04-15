import { useState, useRef } from "react";
import { useTodos, type Filter, type Todo } from "./hooks";

function CheckIcon() {
	return (
		<svg width="12" height="12" viewBox="0 0 12 12" fill="none">
			<path d="M2.5 6L5 8.5L9.5 3.5" stroke="white" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
		</svg>
	);
}

function TodoItem({ todo, onToggle, onDelete, onEdit }: { todo: Todo; onToggle: () => void; onDelete: () => void; onEdit: (text: string) => void }) {
	const [editing, setEditing] = useState(false);
	const [editText, setEditText] = useState(todo.text);
	const inputRef = useRef<HTMLInputElement>(null);

	function startEdit() {
		setEditText(todo.text);
		setEditing(true);
		setTimeout(() => inputRef.current?.select(), 0);
	}

	function commitEdit() {
		const trimmed = editText.trim();
		setEditing(false);
		if (trimmed && trimmed !== todo.text) {
			onEdit(trimmed);
		}
	}

	return (
		<div
			className={`group flex items-center gap-3 rounded-md] border border-(--border) bg-(--surface) px-3.5 py-3 mb-1.5 transition-colors hover:bg-(--surface-hover) ${todo.done ? "opacity-55" : ""}`}
		>
			<button
				onClick={onToggle}
				className={`flex h-5 w-5 shrink-0 items-center justify-center rounded-mdrder-2 transition-colors cursor-pointer ${
					todo.done ? "border(--accent) bg-(--accent)" : "border-(--border) hover:border(--accent)"
				}`}
			>
				{todo.done && <CheckIcon />}
			</button>

			<div className="flex-1 min-w-0" onDoubleClick={startEdit}>
				{editing ? (
					<input
						ref={inputRef}
						value={editText}
						onChange={(e) => setEditText(e.target.value)}
						onBlur={commitEdit}
						onKeyDown={(e) => {
							if (e.key === "Enter") commitEdit();
							if (e.key === "Escape") setEditing(false);
						}}
						className="w-full bg-transparent text-sm text-(--text) outline-none py-0.5"
					/>
				) : (
					<span className={`text-sm leading-relaxed wrap-break-word ${todo.done ? "line-through text-(--text-done)" : ""}`}>{todo.text}</span>
				)}
			</div>

			<button
				onClick={onDelete}
				className="flex h-7 w-7 shrink-0 items-center justify-center rounded-mdxt-base text-(--text-dim) opacity-0 transition-all cursor-pointer group-hover:opacity-100 hover:bg-(--danger-dim) hover:text-(--danger)"
			>
				&times;
			</button>
		</div>
	);
}

export default function App() {
	const { todos, addTodo, toggleTodo, deleteTodo, editTodo, clearCompleted, exportTodos, importTodos } = useTodos();
	const [input, setInput] = useState("");
	const [filter, setFilter] = useState<Filter>("all");

	const doneCount = todos.filter((t) => t.done).length;
	const remaining = todos.length - doneCount;

	const visible = todos.filter((t) => {
		if (filter === "active") return !t.done;
		if (filter === "done") return t.done;
		return true;
	});

	async function handleAdd() {
		const text = input.trim();
		if (!text) return;
		addTodo(text);
		setInput("");
	}

	const filters: { key: Filter; label: string }[] = [
		{ key: "all", label: "All" },
		{ key: "active", label: "Active" },
		{ key: "done", label: "Done" },
	];

	return (
		<div className="flex h-screen flex-col overflow-hidden">
			{/* Header */}
			<div className="shrink-0 px-6 pt-7">
				<h1 className="text-[26px] font-bold tracking-tight text-white">Silk Todo</h1>
				<p className="mt-1 text-[13px] text-(--text-dim)">React + Tailwind &mdash; powered by Silk runtime</p>
				<div className="mt-3.5 flex items-center gap-4 font-mono text-xs text-(--text-dim)">
					<span>
						<span className="font-semibold text-(--accent)">{todos.length}</span> total
					</span>
					<span>
						<span className="font-semibold text(--accent)">{remaining}</span> remaining
					</span>
					<span>
						<span className="font-semibold text(--accent)">{doneCount}</span> done
					</span>
					<span className="ml-auto flex gap-2 font-sans">
						<button
							onClick={importTodos}
							className="cursor-pointer rounded-md] bg-(--accent) px-4.5 text-sm font-semibold text-white transition-opacity hover:opacity-85 active:opacity-70"
						>
							Import
						</button>
						<button
							onClick={exportTodos}
							className="cursor-pointer rounded-md] bg-(--accent) px-4.5 text-sm font-semibold text-white transition-opacity hover:opacity-85 active:opacity-70"
						>
							Export
						</button>
					</span>
				</div>
			</div>

			{/* Input */}
			<div className="flex shrink-0 gap-2 px-6 py-4">
				<input
					value={input}
					onChange={(e) => setInput(e.target.value)}
					onKeyDown={(e) => e.key === "Enter" && handleAdd()}
					placeholder="What needs to be done?"
					className="flex-1 rounded-md] border border-(--border) bg-(--surface) px-3.5 py-2.5 text-sm text-(--text) outline-none transition-colors placeholder:text-(--text-dim) focus:border-(--border-focus)"
				/>
				<button
					onClick={handleAdd}
					className="cursor-pointer rounded-md] bg-(--accent) px-4.5 text-sm font-semibold text-white transition-opacity hover:opacity-85 active:opacity-70"
				>
					Add
				</button>
			</div>

			{/* Filters */}
			<div className="flex shrink-0 gap-1 px-6 pb-3">
				{filters.map(({ key, label }) => (
					<button
						key={key}
						onClick={() => setFilter(key)}
						className={`cursor-pointer rounded-md border px-3 py-1 text-xs font-medium transition-all ${
							filter === key
								? "border-[rgba(153,102,255,0.25)] bg-(--accent-dim) text(--accent)"
								: "border-transparent text-(--text-dim) hover:bg-(--surface) hover:text-(--text)"
						}`}
					>
						{label}
					</button>
				))}
			</div>

			{/* List */}
			<div className="flex-1 overflow-y-auto px-6 pb-6">
				{visible.length === 0 ? (
					<div className="pt-12 text-center text-sm text-(--text-dim)">
						<div className="mb-2 text-3xl opacity-40"></div>
						{todos.length === 0 ? "No todos yet" : filter === "done" ? "Nothing completed" : "All done!"}
					</div>
				) : (
					<>
						{visible.map((todo) => (
							<TodoItem
								key={todo.id}
								todo={todo}
								onToggle={() => toggleTodo(todo.id)}
								onDelete={() => deleteTodo(todo.id)}
								onEdit={(text) => editTodo(todo.id, text)}
							/>
						))}
						{doneCount > 0 && (
							<button
								onClick={clearCompleted}
								className="mx-auto mt-2 block cursor-pointer rounded-md border border-(--border) bg-transparent px-3.5 py-1.5 text-xs text-(--text-dim) transition-all hover:border-(--danger) hover:bg-(--danger-dim) hover:text-(--danger)"
							>
								Clear {doneCount} completed
							</button>
						)}
					</>
				)}
			</div>
		</div>
	);
}
