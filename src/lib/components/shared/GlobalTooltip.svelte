<script lang="ts" module>
	/** Hover longer than the browser's native tooltip before explaining. */
	export const TIP_DELAY_MS = 700;

	const INTERACTIVE = 'button, a[href], [role="button"], [role="tab"], [role="link"], [role="menuitem"], [role="switch"], summary, label, select, input';

	/**
	 * Element whose explanation should show for `node`: the closest one with
	 * `data-tip`, or a clickable one with a native `title` (moved to
	 * `data-tip` so the browser's own tooltip does not show too).
	 */
	export function tipTarget(node: EventTarget | null): HTMLElement | null {
		if (!(node instanceof Element)) return null;
		const tipped = node.closest<HTMLElement>('[data-tip]');
		if (tipped && tipped.dataset.tip?.trim()) return tipped;
		const titled = node.closest<HTMLElement>('[title]');
		if (titled && titled.matches(INTERACTIVE) && titled.title.trim()) {
			titled.dataset.tip = titled.title;
			titled.removeAttribute('title');
			return titled;
		}
		return null;
	}
</script>

<script lang="ts">
	import { onMount, tick } from 'svelte';

	let target = $state<HTMLElement | null>(null);
	let visible = $state(false);
	let pos = $state({ top: 0, left: 0, above: false });
	let box = $state<HTMLDivElement | null>(null);
	let timer: ReturnType<typeof setTimeout> | null = null;
	let pending: HTMLElement | null = null;

	const title = $derived(target?.dataset.tipTitle ?? '');
	const text = $derived(target?.dataset.tip ?? '');

	function clear() {
		if (timer) clearTimeout(timer);
		timer = null;
		pending = null;
	}

	function hide() {
		clear();
		if (target) target.removeAttribute('aria-describedby');
		visible = false;
		target = null;
	}

	async function show(el: HTMLElement) {
		target = el;
		visible = true;
		el.setAttribute('aria-describedby', 'app-tip');
		await tick();
		place();
	}

	function place() {
		if (!target || !box) return;
		const r = target.getBoundingClientRect();
		const b = box.getBoundingClientRect();
		const gap = 8;
		const above = r.bottom + gap + b.height > window.innerHeight - 8;
		const top = above ? r.top - gap - b.height : r.bottom + gap;
		let left = r.left + r.width / 2 - b.width / 2;
		left = Math.max(8, Math.min(left, window.innerWidth - b.width - 8));
		pos = { top: Math.max(8, top), left, above };
	}

	function schedule(el: HTMLElement) {
		// moving between the icon and the text of the same button keeps the count
		if ((el === target && visible) || el === pending) return;
		hide();
		pending = el;
		timer = setTimeout(() => {
			pending = null;
			timer = null;
			show(el);
		}, TIP_DELAY_MS);
	}

	onMount(() => {
		const over = (e: PointerEvent) => {
			if (e.pointerType === 'touch') return;
			const el = tipTarget(e.target);
			if (el) schedule(el);
			else if (!(e.target instanceof Node && box?.contains(e.target))) hide();
		};
		const focus = (e: FocusEvent) => {
			const el = tipTarget(e.target);
			// keyboard focus only (a click also focuses: no tip on click)
			if (el && el.matches(':focus-visible')) schedule(el);
		};
		const key = (e: KeyboardEvent) => {
			if (e.key === 'Escape' && (visible || timer)) hide();
		};
		document.addEventListener('pointerover', over);
		document.addEventListener('focusin', focus);
		document.addEventListener('focusout', hide);
		document.addEventListener('pointerdown', hide, true);
		document.addEventListener('keydown', key);
		window.addEventListener('scroll', hide, true);
		window.addEventListener('blur', hide);
		return () => {
			clear();
			document.removeEventListener('pointerover', over);
			document.removeEventListener('focusin', focus);
			document.removeEventListener('focusout', hide);
			document.removeEventListener('pointerdown', hide, true);
			document.removeEventListener('keydown', key);
			window.removeEventListener('scroll', hide, true);
			window.removeEventListener('blur', hide);
		};
	});
</script>

{#if visible && text}
	<div
		bind:this={box}
		id="app-tip"
		role="tooltip"
		class="app-tip"
		style="top: {pos.top}px; left: {pos.left}px;"
	>
		{#if title}<p class="app-tip-title">{title}</p>{/if}
		<p class="app-tip-text">{text}</p>
	</div>
{/if}

<style>
	.app-tip {
		position: fixed;
		z-index: 10000;
		max-width: 320px;
		padding: 8px 11px;
		border-radius: 8px;
		pointer-events: none;
		font-size: 12.5px;
		line-height: 1.45;
		background: #ffffff;
		color: #1f2937;
		border: 1px solid #e5e7eb;
		box-shadow: 0 6px 20px rgba(0, 0, 0, 0.12);
		animation: tip-in 90ms ease-out;
	}
	:global(.dark) .app-tip {
		background: #111827;
		color: #e5e7eb;
		border-color: #374151;
		box-shadow: 0 6px 20px rgba(0, 0, 0, 0.45);
	}
	.app-tip-title {
		font-weight: 600;
		margin-bottom: 2px;
	}
	.app-tip-text {
		white-space: pre-line;
	}
	@keyframes tip-in {
		from {
			opacity: 0;
		}
		to {
			opacity: 1;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.app-tip {
			animation: none;
		}
	}
</style>
