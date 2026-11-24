<script lang="ts">
	import { i18n } from '$lib/i18n';

	type Props = { diff: string };
	let { diff }: Props = $props();

	let lines = $derived(diff ? diff.replace(/\n$/, '').split('\n') : []);

	function cls(line: string): string {
		if (line.startsWith('+++') || line.startsWith('---')) return 'text-gray-500 dark:text-gray-400';
		if (line.startsWith('@@')) return 'text-primary-600 dark:text-primary-300';
		if (line.startsWith('+')) return 'bg-emerald-50 text-emerald-800 dark:bg-emerald-950/50 dark:text-emerald-200';
		if (line.startsWith('-')) return 'bg-red-50 text-red-800 dark:bg-red-950/50 dark:text-red-200';
		return 'text-gray-600 dark:text-gray-300';
	}
</script>

{#if lines.length === 0}
	<p class="text-meta">{i18n.t('sync.diff.empty')}</p>
{:else}
	<pre class="font-mono text-[12px] leading-5 overflow-x-auto rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-950 py-2"
		aria-label={i18n.t('sync.diff.label')}>{#each lines as line, i (i)}<div class="px-3 whitespace-pre {cls(line)}"><span class="sr-only">{line.startsWith('+') && !line.startsWith('+++') ? i18n.t('sync.diff.added') : line.startsWith('-') && !line.startsWith('---') ? i18n.t('sync.diff.removed') : ''}</span>{line}</div>{/each}</pre>
	<p class="text-meta mt-2">{i18n.t('sync.diff.masked')}</p>
{/if}
