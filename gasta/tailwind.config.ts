import { join } from 'path';
import type { Config } from 'tailwindcss';
import forms from '@tailwindcss/forms';
import { DTheme } from './d-theme';

const config = {
	darkMode: 'class',
	content: ['./src/**/*.{html,js,svelte,ts}', join('../**/*.{html,js,svelte,ts}')],
	theme: {
		extend: {
			keyframes: {
				'collapsible-down': {
					from: { height: '0' },
					to: { height: 'var(--radix-collapsible-content-height)' } // or var(--bits-accordion-content-height)
				},
				'collapsible-up': {
					from: { height: 'var(--radix-collapsible-content-height)' }, // or var(--bits-accordion-content-height)
					to: { height: '0' }
				}
			},
			animation: {
				'collapsible-down': 'collapsible-down 0.2s ease-out',
				'collapsible-up': 'collapsible-up 0.2s ease-out'
			}
		}
	},
	plugins: [forms]
} satisfies Config;

export default config;
