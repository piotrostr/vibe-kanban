// vite.config.ts
import { sentryVitePlugin } from "@sentry/vite-plugin";
import { defineConfig, Plugin } from "vite";
import react from "@vitejs/plugin-react";
import { VitePWA } from "vite-plugin-pwa";
import path from "path";
import fs from "fs";

function executorSchemasPlugin(): Plugin {
	const VIRTUAL_ID = "virtual:executor-schemas";
	const RESOLVED_VIRTUAL_ID = "\0" + VIRTUAL_ID;

	return {
		name: "executor-schemas-plugin",
		resolveId(id) {
			if (id === VIRTUAL_ID) return RESOLVED_VIRTUAL_ID; // keep it virtual
			return null;
		},
		load(id) {
			if (id !== RESOLVED_VIRTUAL_ID) return null;

			const schemasDir = path.resolve(__dirname, "../shared/schemas");
			const files = fs.existsSync(schemasDir)
				? fs.readdirSync(schemasDir).filter((f) => f.endsWith(".json"))
				: [];

			const imports: string[] = [];
			const entries: string[] = [];

			files.forEach((file, i) => {
				const varName = `__schema_${i}`;
				const importPath = `shared/schemas/${file}`; // uses your alias
				const key = file.replace(/\.json$/, "").toUpperCase(); // claude_code -> CLAUDE_CODE
				imports.push(`import ${varName} from "${importPath}";`);
				entries.push(`  "${key}": ${varName}`);
			});

			// IMPORTANT: pure JS (no TS types), and quote keys.
			const code = `
${imports.join("\n")}

export const schemas = {
${entries.join(",\n")}
};

export default schemas;
`;
			return code;
		},
	};
}

export default defineConfig({
	plugins: [
		react(),
		sentryVitePlugin({ org: "bloop-ai", project: "vibe-kanban" }),
		executorSchemasPlugin(),
		VitePWA({
			registerType: "autoUpdate",
			includeAssets: ["vibe-192.png", "vibe-512.png", "vibe-apple-touch.png"],
			manifest: false, // Use existing site.webmanifest
			workbox: {
				globPatterns: ["**/*.{js,css,html,ico,png,svg,woff2}"],
				runtimeCaching: [
					{
						urlPattern: /^https:\/\/fonts\.googleapis\.com\/.*/i,
						handler: "CacheFirst",
						options: {
							cacheName: "google-fonts-cache",
							expiration: {
								maxEntries: 10,
								maxAgeSeconds: 60 * 60 * 24 * 365, // 1 year
							},
							cacheableResponse: {
								statuses: [0, 200],
							},
						},
					},
				],
			},
			devOptions: {
				enabled: true,
			},
		}),
	],
	resolve: {
		alias: {
			"@": path.resolve(__dirname, "./src"),
			shared: path.resolve(__dirname, "../shared"),
		},
	},
	server: {
		port: parseInt(process.env.FRONTEND_PORT || "6769"),
		proxy: {
			"/api": {
				target: `http://localhost:${process.env.BACKEND_PORT || "6770"}`,
				changeOrigin: true,
				ws: true,
			},
		},
		fs: {
			allow: [path.resolve(__dirname, "."), path.resolve(__dirname, "..")],
		},
		open: process.env.VITE_OPEN === "true",
	},
	optimizeDeps: {
		exclude: ["wa-sqlite"],
	},
	build: { sourcemap: true },
});
