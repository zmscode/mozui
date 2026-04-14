import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import silk from "./plugins/silk-vite-plugin";

export default defineConfig({
	plugins: [silk(), react(), tailwindcss()],
});
