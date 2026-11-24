<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { Trees, ExternalLink, CircleCheck, CircleDashed } from 'lucide-svelte';
	import { Header } from '$lib/components/layout';
	import { notifications } from '$lib/stores';
	import { i18n } from '$lib/i18n';

	type Status = { installed: boolean; dir: string | null; url: string; up: boolean };
	let status = $state<Status | null>(null);
	let opening = $state(false);

	async function refresh() {
		try {
			status = await invoke<Status>('code_forest_status');
		} catch (e) {
			notifications.add('error', i18n.t('forest.failed'), { detail: String(e) });
		}
	}

	async function open() {
		opening = true;
		try {
			await invoke('open_code_forest');
			await refresh();
		} catch (e) {
			notifications.add('error', i18n.t('forest.failed'), { detail: String(e), duration: 10000 });
		} finally {
			opening = false;
		}
	}

	onMount(async () => {
		await refresh();
		// the tab exists to see the forest: open it right away
		if (status?.installed) open();
	});
</script>

<Header title="Code Forest" subtitle={i18n.t('forest.subtitle')} />

<div class="flex-1 overflow-y-auto">
	<div class="max-w-3xl mx-auto px-6 py-8">
		<section class="card space-y-4">
			<div class="flex items-start gap-4">
				<Trees class="w-8 h-8 text-emerald-500 shrink-0" aria-hidden="true" />
				<div class="flex-1 space-y-1">
					<h2 class="text-title">{i18n.t('forest.title')}</h2>
					<p class="text-body">{i18n.t('forest.body')}</p>
				</div>
			</div>
			{#if status && !status.installed}
				<p class="text-body">{i18n.t('forest.notInstalled')}</p>
			{:else if status}
				<p class="text-meta flex items-center gap-1.5">
					{#if status.up}<CircleCheck class="w-4 h-4 text-emerald-500" aria-hidden="true" />{i18n.t('forest.up', { url: status.url })}
					{:else}<CircleDashed class="w-4 h-4 text-gray-400" aria-hidden="true" />{i18n.t('forest.down')}{/if}
				</p>
				<div class="flex items-center gap-3">
					<button class="btn btn-primary" disabled={opening} onclick={open}>
						<ExternalLink class="w-4 h-4 mr-1.5" aria-hidden="true" />{opening ? i18n.t('forest.opening') : i18n.t('forest.open')}
					</button>
					<span class="text-meta">{i18n.t('forest.windowHint')}</span>
				</div>
			{/if}
		</section>
	</div>
</div>
