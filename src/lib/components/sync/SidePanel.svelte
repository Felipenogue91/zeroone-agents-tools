<script lang="ts">
	import { X } from 'lucide-svelte';
	import type { Snippet } from 'svelte';
	import { i18n } from '$lib/i18n';

	type Props = {
		open: boolean;
		title: string;
		subtitle?: string;
		onClose: () => void;
		children: Snippet;
		footer?: Snippet;
	};

	let { open, title, subtitle, onClose, children, footer }: Props = $props();
	let panel = $state<HTMLDivElement | null>(null);

	$effect(() => {
		if (open) panel?.focus();
	});

	function onKey(e: KeyboardEvent) {
		if (open && e.key === 'Escape') {
			e.stopPropagation();
			onClose();
		}
	}
</script>

<svelte:window onkeydown={onKey} />

{#if open}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="fixed inset-0 z-40 bg-black/30" onclick={onClose}></div>
	<div
		bind:this={panel}
		tabindex="-1"
		role="dialog"
		aria-modal="true"
		aria-label={title}
		class="fixed right-0 top-0 bottom-0 z-50 w-full max-w-xl flex flex-col bg-white dark:bg-gray-800 border-l border-gray-200 dark:border-gray-700 shadow-xl outline-none"
	>
		<header class="flex items-start gap-3 px-6 py-4 border-b border-gray-200 dark:border-gray-700">
			<div class="flex-1 min-w-0">
				<h2 class="text-title truncate">{title}</h2>
				{#if subtitle}<p class="text-meta mt-1 break-all">{subtitle}</p>{/if}
			</div>
			<button class="btn btn-ghost p-2" onclick={onClose} aria-label={i18n.t('sync.panel.close')} title={i18n.t('sync.panel.close')}>
				<X class="w-4 h-4" aria-hidden="true" />
			</button>
		</header>
		<div class="flex-1 overflow-y-auto px-6 py-4 space-y-4">
			{@render children()}
		</div>
		{#if footer}
			<footer class="px-6 py-3 border-t border-gray-200 dark:border-gray-700 flex items-center justify-end gap-2">
				{@render footer()}
			</footer>
		{/if}
	</div>
{/if}
