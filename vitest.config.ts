import { defineConfig } from 'vitest/config';

export default defineConfig({
	test: {
		globals: true,
		coverage: {
			enabled: false
		}
	},
	esbuild: {
		target: 'es2020'
	}
});
