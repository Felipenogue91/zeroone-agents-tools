<script lang="ts">
	import { onMount } from 'svelte';
	import { Header } from '$lib/components/layout';
	import { syncStore } from '$lib/stores/syncStore.svelte';
	import { i18n } from '$lib/i18n';
	import ResourceList from '$lib/components/sync/ResourceList.svelte';
	import ResourceDetail from '$lib/components/sync/ResourceDetail.svelte';
	import AddMcpPanel from '$lib/components/sync/AddMcpPanel.svelte';
	import type { ResourceView } from '$lib/types/sync';

	let selectedUid = $state<string | null>(null);
	let adding = $state(false);

	// keep the open panel in sync with refreshed data
	let selected = $derived(syncStore.resources.find((r) => r.uid === selectedUid) ?? null);

	let managedProjects = $derived(
		[...new Set(syncStore.resources.filter((r) => r.scope.type === 'project').map((r) => (r.scope as { path: string }).path))].sort()
	);

	onMount(() => {
		const params = new URLSearchParams(window.location.search);
		if (params.get('add')) adding = true;
		const openUid = params.get('open');
		if (openUid) selectedUid = openUid;
		syncStore.loadResources();
		if (!syncStore.health) syncStore.loadHealth();
	});

	function open(r: ResourceView) {
		selectedUid = r.uid;
	}
</script>

<Header title={i18n.t('sync.lib.title')} subtitle={i18n.t('sync.lib.subtitle')} />

<div class="flex-1 overflow-hidden">
	<ResourceList resources={syncStore.resources} loading={syncStore.isLoadingResources} onOpen={open} onAdd={() => (adding = true)} />
</div>

<ResourceDetail resource={selected} onClose={() => (selectedUid = null)} />
<AddMcpPanel open={adding} projects={managedProjects} onClose={() => (adding = false)} />
