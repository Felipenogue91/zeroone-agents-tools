<script lang="ts">
	import { Trash2, Pencil, FileText } from 'lucide-svelte';
	import { syncStore } from '$lib/stores/syncStore.svelte';
	import { i18n } from '$lib/i18n';
	import type { ResourceView } from '$lib/types/sync';
	import { CLIS, CLI_LABEL, scopeLabel } from '$lib/types/sync';
	import CliChips from './CliChips.svelte';
	import CliChip from './CliChip.svelte';
	import SidePanel from './SidePanel.svelte';

	type Props = { resource: ResourceView | null; onClose: () => void };
	let { resource, onClose }: Props = $props();

	let editing = $state(false);
	let draft = $state('');
	let draftError = $state<string | null>(null);
	let confirmDelete = $state(false);

	$effect(() => {
		// reset when another resource opens
		resource?.uid;
		editing = false;
		confirmDelete = false;
		draftError = null;
	});

	let project = $derived(resource?.scope.type === 'project' ? resource.scope.path : null);

	function startEdit() {
		if (!resource?.config) return;
		draft = JSON.stringify(resource.config, null, 2);
		draftError = null;
		editing = true;
	}

	async function saveEdit() {
		if (!resource) return;
		let parsed: Record<string, unknown>;
		try {
			parsed = JSON.parse(draft);
			if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) throw new Error('obj');
		} catch {
			draftError = i18n.t('sync.detail.invalidJson');
			return;
		}
		if (JSON.stringify(parsed).includes('••••••')) {
			draftError = i18n.t('sync.detail.maskedValue');
			return;
		}
		const out = await syncStore.setServer(resource.name, parsed, project);
		if (out) {
			editing = false;
			onClose();
		}
	}

	async function remove() {
		if (!resource) return;
		const out = await syncStore.setServer(resource.name, null, project);
		if (out) onClose();
	}
</script>

<SidePanel open={!!resource} title={resource?.name ?? ''} subtitle={resource ? `${i18n.t(`sync.lib.kind.${resource.kind}` as never)} · ${scopeLabel(resource.scope)}` : ''} {onClose}>
	{#if resource}
		<section class="space-y-2">
			<h3 class="text-meta uppercase tracking-wide font-semibold">{i18n.t('sync.detail.where')}</h3>
			<CliChips targets={resource.targets}
				onToggle={resource.kind === 'agent' ? undefined : (cli, enabled) =>
					resource.kind === 'mcp' ? syncStore.setTarget(resource.name, cli, enabled, project) : syncStore.setSkillTarget(resource.name, cli, enabled)} />
			{#if resource.description}<p class="text-body">{resource.description}</p>{/if}
		</section>

		<section class="space-y-1">
			<h3 class="text-meta uppercase tracking-wide font-semibold">{i18n.t('sync.detail.source')}</h3>
			<p class="text-body font-mono text-xs break-all flex items-start gap-1.5"><FileText class="w-3.5 h-3.5 mt-0.5 shrink-0 text-gray-400" aria-hidden="true" />{resource.canonical}</p>
			{#if resource.transport}<p class="text-meta">{i18n.t('sync.detail.transport')}: <span class="font-mono">{resource.transport}</span></p>{/if}
		</section>

		{#if resource.config}
			<section class="space-y-2">
				<div class="flex items-center">
					<h3 class="text-meta uppercase tracking-wide font-semibold flex-1">{i18n.t('sync.detail.config')}</h3>
					{#if !editing && resource.kind === 'mcp'}
						<button class="btn btn-ghost text-xs px-2 py-1" onclick={startEdit}><Pencil class="w-3 h-3 mr-1" aria-hidden="true" />{i18n.t('sync.detail.edit')}</button>
					{/if}
				</div>
				{#if editing}
					<label class="block">
						<span class="sr-only">{i18n.t('sync.detail.config')}</span>
						<textarea class="input font-mono text-xs h-56" bind:value={draft} spellcheck="false"></textarea>
					</label>
					{#if draftError}<p class="text-xs text-red-600 dark:text-red-400" role="alert">{draftError}</p>{/if}
					<p class="text-meta">{i18n.t('sync.detail.secretsHint')}</p>
					<div class="flex gap-2 justify-end">
						<button class="btn btn-ghost" onclick={() => (editing = false)}>{i18n.t('sync.panel.cancel')}</button>
						<button class="btn btn-primary" onclick={saveEdit}>{i18n.t('sync.detail.save')}</button>
					</div>
				{:else}
					<pre class="font-mono text-xs bg-gray-50 dark:bg-gray-900 rounded-lg p-3 overflow-x-auto">{JSON.stringify(resource.config, null, 2)}</pre>
				{/if}
			</section>
		{/if}

		{#if resource.secretRefs.length || resource.literalSecretKeys.length}
			<section class="space-y-1">
				<h3 class="text-meta uppercase tracking-wide font-semibold">{i18n.t('sync.detail.secrets')}</h3>
				{#if resource.secretRefs.length}
					<p class="text-body">{i18n.t('sync.detail.refs')} {#each resource.secretRefs as v (v)}<code class="font-mono text-xs mr-1">{'${' + v + '}'}</code>{/each}</p>
				{/if}
				{#if resource.literalSecretKeys.length}
					<p class="text-body text-amber-700 dark:text-amber-300">{i18n.t('sync.detail.literal', { keys: resource.literalSecretKeys.join(', ') })}</p>
				{/if}
			</section>
		{/if}

		{#if Object.keys(resource.overrides).length}
			<section class="space-y-1">
				<h3 class="text-meta uppercase tracking-wide font-semibold">{i18n.t('sync.detail.overrides')}</h3>
				{#each CLIS.filter((c) => resource.overrides[c]) as cli (cli)}
					<p class="text-body">{CLI_LABEL[cli]}: <span class="font-mono text-xs">{Object.keys(resource.overrides[cli] ?? {}).join(', ')}</span></p>
				{/each}
			</section>
		{/if}

		<section class="space-y-1">
			<h3 class="text-meta uppercase tracking-wide font-semibold">{i18n.t('sync.detail.seenIn')}</h3>
			{#if resource.occurrences.length === 0}
				<p class="text-meta">{i18n.t('sync.detail.notSeen')}</p>
			{:else}
				<ul class="space-y-1">
					{#each resource.occurrences as o (o.cli + o.filePath)}
						<li class="flex items-center gap-2"><CliChip cli={o.cli} label /><span class="text-meta break-all">{o.filePath}</span></li>
					{/each}
				</ul>
			{/if}
		</section>
	{/if}

	{#snippet footer()}
		{#if resource?.kind === 'mcp'}
			{#if confirmDelete}
				<span class="text-body mr-auto">{i18n.t('sync.detail.confirmRemove')}</span>
				<button class="btn btn-ghost" onclick={() => (confirmDelete = false)}>{i18n.t('sync.panel.cancel')}</button>
				<button class="btn bg-red-600 text-white hover:bg-red-700" onclick={remove}>{i18n.t('sync.detail.remove')}</button>
			{:else}
				<button class="btn btn-ghost text-red-600 dark:text-red-400 mr-auto" onclick={() => (confirmDelete = true)}>
					<Trash2 class="w-4 h-4 mr-1.5" aria-hidden="true" />{i18n.t('sync.detail.remove')}
				</button>
				<button class="btn btn-secondary" onclick={onClose}>{i18n.t('sync.panel.close')}</button>
			{/if}
		{:else}
			<button class="btn btn-secondary" onclick={onClose}>{i18n.t('sync.panel.close')}</button>
		{/if}
	{/snippet}
</SidePanel>
