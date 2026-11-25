<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { Trees, RotateCw, LoaderCircle } from 'lucide-svelte';
	import { Header } from '$lib/components/layout';
	import { i18n } from '$lib/i18n';

	type Status = { installed: boolean; dir: string | null; url: string; up: boolean };
	let status = $state<Status | null>(null);
	let starting = $state(false);
	let error = $state<string | null>(null);
	// the iframe only mounts once the server answers, so it never shows a dead page
	let ready = $state(false);

	async function start() {
		starting = true;
		error = null;
		try {
			status = await invoke<Status>('code_forest_status');
			if (!status.installed) return;
			if (!status.up) await invoke('start_code_forest');
			ready = true;
		} catch (e) {
			error = String(e);
		} finally {
			starting = false;
		}
	}

	onMount(start);
</script>

<Header title="Code Forest" subtitle={i18n.t('forest.subtitle')} />

{#if ready && status}
	<iframe
		src={status.url}
		title="Code Forest"
		class="flex-1 min-h-0 w-full border-0 bg-gray-950"
	></iframe>
{:else}
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
				{:else if error}
					<p class="text-body text-red-500" role="alert">{i18n.t('forest.failed')}: {error}</p>
					<button data-tip={i18n.t('tip.forest.retry')} class="btn btn-primary" onclick={start}>
						<RotateCw class="w-4 h-4 mr-1.5" aria-hidden="true" />{i18n.t('forest.retry')}
					</button>
				{:else}
					<p class="text-meta flex items-center gap-1.5" role="status">
						<LoaderCircle class="w-4 h-4 animate-spin" aria-hidden="true" />{i18n.t('forest.starting')}
					</p>
				{/if}
			</section>
		</div>
	</div>
{/if}
