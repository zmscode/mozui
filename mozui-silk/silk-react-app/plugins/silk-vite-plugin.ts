import type { Plugin } from "vite";

/**
 * Silk Vite Plugin
 *
 * Integrates Vite with the Silk runtime:
 * - Dev: configures CORS and disables browser open
 * - Build: outputs to app/ with relative paths for mozui:// serving,
 *   removes crossorigin attributes that break custom protocols
 */
export default function silk(): Plugin {
	return {
		name: "silk",

		config(_config, { command }) {
			if (command === "build") {
				return {
					build: {
						outDir: "app",
						emptyOutDir: true,
						// Disable crossorigin attributes — mozui:// is same-origin
						crossOriginLoading: false,
					},
					// Relative paths so mozui:// protocol resolves assets
					base: "./",
				};
			}
			return {
				server: {
					open: false,
					cors: true,
				},
			};
		},

		// Strip crossorigin from link tags in production HTML
		transformIndexHtml(html) {
			return html.replace(/ crossorigin/g, "");
		},
	};
}
