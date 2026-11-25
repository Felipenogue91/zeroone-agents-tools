import { describe, it, expect } from 'vitest';
import { tipTarget, TIP_DELAY_MS } from '$lib/components/shared/GlobalTooltip.svelte';

function html(markup: string): HTMLElement {
	const root = document.createElement('div');
	root.innerHTML = markup;
	document.body.append(root);
	return root;
}

describe('GlobalTooltip.tipTarget', () => {
	it('waits longer than the native tooltip', () => {
		expect(TIP_DELAY_MS).toBeGreaterThanOrEqual(600);
	});

	it('finds the explained element from an inner icon', () => {
		const root = html('<button data-tip="Explica"><svg><path></path></svg><span>Ok</span></button>');
		const icon = root.querySelector('path')!;
		expect(tipTarget(icon)).toBe(root.querySelector('button'));
	});

	it('turns the native title of a clickable element into an explanation', () => {
		const root = html('<a href="/x" title="Abre a aba">x</a>');
		const a = root.querySelector('a')!;
		expect(tipTarget(a)).toBe(a);
		expect(a.dataset.tip).toBe('Abre a aba');
		expect(a.hasAttribute('title')).toBe(false); // no double tooltip
	});

	it('leaves the title of plain text alone', () => {
		const root = html('<p title="/caminho/longo">/caminho</p>');
		expect(tipTarget(root.querySelector('p'))).toBeNull();
		expect(root.querySelector('p')!.title).toBe('/caminho/longo');
	});

	it('drops the native title when there is an explanation', () => {
		const root = html('<button data-tip="Longa" title="Curta">x</button>');
		const b = root.querySelector('button')!;
		expect(tipTarget(b)).toBe(b);
		expect(b.hasAttribute('title')).toBe(false);
	});

	it('ignores empty explanations', () => {
		const root = html('<button data-tip="  ">x</button>');
		expect(tipTarget(root.querySelector('button'))).toBeNull();
	});
});
