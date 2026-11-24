<script lang="ts">
	import { onMount } from 'svelte';
	import { Header } from '$lib/components/layout';
	import { syncStore } from '$lib/stores/syncStore.svelte';
	import { i18n } from '$lib/i18n';
	import ResourceMatrix from '$lib/components/sync/ResourceMatrix.svelte';
	import type { ResourceKind } from '$lib/types/sync';
	import ResourceDetail from '$lib/components/sync/ResourceDetail.svelte';
	import AddMcpPanel from '$lib/components/sync/AddMcpPanel.svelte';
	import type { ResourceView } from '$lib/types/sync';

	let selectedUid = $state<string | null>(null);
	let kind = $state<ResourceKind>('mcp');
	const kinds: { id: ResourceKind; key: string }[] = [
		{ id: 'mcp', key: 'sync.lib.kindMcp' },
		{ id: 'skill', key: 'sync.lib.kindSkill' }
	];
	let counts = $derived({
		mcp: syncStore.resources.filter((r) => r.kind === 'mcp').length,
		skill: syncStore.resources.filter((r) => r.kind === 'skill').length
	});
	let adding = $state(false);

	// keep the open panel in sync with refreshed data
	let selected = $derived(syncStore.resources.find((r) => r.uid === selectedUid) ?? null);

	let managedProjects = $derived(
		[...new Set(syncStore.resources.filter((r) => r.scope.type === 'project').map((r) => (r.scope as { path: string }).path))].sort()
	);

	onMount(() => {
		const params = new URLSearchParams(window.location.search);
		if (params.get('add')) adding = true;
		const k = params.get('kind');
		if (k === 'skill' || k === 'mcp') kind = k;
		const openUid = params.get('open');
		if (openUid) selectedUid = openUid;
		syncStore.loadResources();
		if (!syncStore.health) syncStore.loadHealth();
		if (!syncStore.live && !syncStore.isLoadingLive) syncStore.loadLive();
	});

	function open(r: ResourceView) {
		selectedUid = r.uid;
	}
</script>

<Header title={i18n.t('sync.lib.title')} subtitle={i18n.t('sync.lib.subtitle')} />

<div role="tablist" aria-label={i18n.t('sync.lib.kindLabel')} class="flex items-center gap-1 px-6 pt-3 bg-white dark:bg-gray-800">
	{#each kinds as k (k.id)}
		<button role="tab" aria-selected={kind === k.id}
			class="px-3 py-1.5 rounded-lg text-sm font-medium transition-colors {kind === k.id
				? 'bg-primary-50 text-primary-700 dark:bg-primary-900/50 dark:text-primary-200'
				: 'text-gray-600 hover:bg-gray-100 dark:text-gray-300 dark:hover:bg-gray-700/60'}"
			onclick={() => (kind = k.id)}>
			{i18n.t(k.key as never)} <span class="text-meta">{counts[k.id as 'mcp' | 'skill']}</span>
		</button>
	{/each}
</div>

<div class="flex-1 overflow-hidden">
	{#key kind}
		<ResourceMatrix {kind} resources={syncStore.resources} loading={syncStore.isLoadingResources} onOpen={open} onAdd={kind === 'mcp' ? () => (adding = true) : undefined} />
	{/key}
</div>

<ResourceDetail resource={selected} onClose={() => (selectedUid = null)} />
<AddMcpPanel open={adding} projects={managedProjects} onClose={() => (adding = false)} />
