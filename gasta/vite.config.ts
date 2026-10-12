import adapter from '@sveltejs/adapter-node';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig({
	plugins: [
		{
			name: 'sveltekit-3-base-shim',
			transform(code, id) {
				if (id.includes('@sveltejs/kit/src/runtime/app/paths/index.js')) {
					return `${code}\nexport { base, assets } from '#app/paths';\n`;
				}
			}
		},
		sveltekit({
			adapter: adapter(),
			compilerOptions: {
				experimental: {
					async: true
				}
			},
			experimental: {
				remoteFunctions: true
			}
		}),
		tailwindcss()
	]
});
