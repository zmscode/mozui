// src/main.ts
function uid() {
  return Math.random().toString(36).slice(2) + Date.now().toString(36);
}
Silk.onReady(() => {
  let todos = [];
  Silk.createWindow("main", {
    url: "./index.html",
    title: "Silk Todo",
    width: 460,
    height: 680,
    minWidth: 360,
    minHeight: 480
  });
  Silk.handle("list-todos", () => todos);
  Silk.handle("add-todo", (args) => {
    const text = args.text?.trim();
    if (!text)
      return null;
    const todo = {
      id: uid(),
      text,
      done: false
    };
    todos.push(todo);
    Silk.emitAll("todos:changed", todos);
    return todo;
  });
  Silk.handle("toggle-todo", (args) => {
    const todo = todos.find((t) => t.id === args.id);
    if (!todo)
      return null;
    todo.done = !todo.done;
    Silk.emitAll("todos:changed", todos);
    return todo;
  });
  Silk.handle("delete-todo", (args) => {
    todos = todos.filter((t) => t.id !== args.id);
    Silk.emitAll("todos:changed", todos);
    return true;
  });
  Silk.handle("edit-todo", (args) => {
    const newText = args.text?.trim();
    if (!newText)
      return null;
    const todo = todos.find((t) => t.id === args.id);
    if (!todo)
      return null;
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
