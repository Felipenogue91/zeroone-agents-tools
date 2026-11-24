<script lang="ts">
	import { Plug, Sparkles, Bot, Webhook, Search, TriangleAlert, Star, Plus, X } from 'lucide-svelte';
	import { syncStore } from '$lib/stores/syncStore.svelte';
	import { i18n } from '$lib/i18n';
	import type { Cli, ResourceKind, ResourceView } from '$lib/types/sync';
	import { CLIS, CLI_SHORT, scopeLabel, scopeKey } from '$lib/types/sync';
	import CliChips from './CliChips.svelte';
	import Skeleton from './Skeleton.svelte';

	type Props = {
		resources: ResourceView[];
		loading?: boolean;
		onOpen: (r: ResourceView) => void;
		onAdd?: () => void;
	};

	let { resources, loading = false, onOpen, onAdd }: Props = $props();

	let query = $state('');
	let kind = $state<'all' | ResourceKind>('all');
	let activeIn = $state<'any' | Cli>('any');
	let origin = $state<'any' | 'global' | 'project'>('any');
	let onlyProblems = $state(false);

	const kindIcon = { mcp: Plug, skill: Sparkles, agent: Bot, hook: Webhook };

	let problemUids = $derived.by(() => {
		const set = new Set<string>();
		for (const d of syncStore.health?.divergences ?? []) {
			if (d.kind === 'agySkillsFolder') continue;
			set.add(`mcp:${scopeKey(d.scope)}:${d.name}`);
		}
		return set;
	});

	function isActive(r: ResourceView, cli: Cli) {
		const s = r.targets[cli]?.state;
		return s === 'on' || s === 'native' || s === 'providedByPlugin';
	}

	let filtered = $derived.by(() => {
		const q = query.trim().toLowerCase();
		return resources.filter((r) => {
			if (kind !== 'all' && r.kind !== kind) return false;
			if (origin !== 'any' && r.scope.type !== origin) return false;
			if (activeIn !== 'any' && !isActive(r, activeIn)) return false;
			if (onlyProblems && !problemUids.has(r.uid) && r.literalSecretKeys.length === 0) return false;
			if (q && !`${r.name} ${r.description ?? ''} ${scopeLabel(r.scope)}`.toLowerCase().includes(q)) return false;
			return true;
		});
	});

	let counts = $derived({
		all: resources.length,
		mcp: resources.filter((r) => r.kind === 'mcp').length,
		skill: resources.filter((r) => r.kind === 'skill').length,
		agent: resources.filter((r) => r.kind === 'agent').length
	});

	let hasFilters = $derived(query !== '' || activeIn !== 'any' || origin !== 'any' || onlyProblems);

	function clearFilters() {
		query = '';
		activeIn = 'any';
		origin = 'any';
		onlyProblems = false;
	}

	function toggle(r: ResourceView, cli: Cli, enabled: boolean) {
		if (r.kind === 'mcp') {
			syncStore.setTarget(r.name, cli, enabled, r.scope.type === 'project' ? r.scope.path : null);
		} else if (r.kind === 'skill') {
			syncStore.setSkillTarget(r.name, cli, enabled);
		}
	}

	async function toggleFavorite(r: ResourceView, e: MouseEvent) {
		e.stopPropagation();
		const { invoke } = await import('@tauri-apps/api/core');
		await invoke('set_resource_favorite', { uid: r.uid, favorite: !r.isFavorite });
		await syncStore.loadResources();
	}

	const kinds: { id: 'all' | ResourceKind; key: string }[] = [
		{ id: 'all', key: 'sync.lib.kindAll' },
		{ id: 'mcp', key: 'sync.lib.kindMcp' },
		{ id: 'skill', key: 'sync.lib.kindSkill' },
		{ id: 'agent', key: 'sync.lib.kindAgent' }
	];
</script>

<div class="flex flex-col h-full">
	<!-- Toolbar -->
	<div class="px-6 py-3 border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 space-y-3">
		<div class="flex items-center gap-2 flex-wrap">
			<div role="tablist" aria-label={i18n.t('sync.lib.kindLabel')} class="flex items-center gap-1">
				{#each kinds as k (k.id)}
					<button role="tab" aria-selected={kind === k.id}
						class="px-3 py-1.5 rounded-lg text-sm font-medium transition-colors {kind === k.id
							? 'bg-primary-50 text-primary-700 dark:bg-primary-900/50 dark:text-primary-200'
							: 'text-gray-600 hover:bg-gray-100 dark:text-gray-300 dark:hover:bg-gray-700/60'}"
						onclick={() => (kind = k.id)}>
						{i18n.t(k.key as never)} <span class="text-meta">{counts[k.id as keyof typeof counts] ?? ''}</span>
					</button>
				{/each}
			</div>
			<div class="flex-1"></div>
			{#if onAdd}
				<button class="btn btn-primary" onclick={onAdd}>
					<Plus class="w-4 h-4 mr-1.5" aria-hidden="true" />
					{i18n.t('sync.lib.addMcp')}
				</button>
			{/if}
		</div>
		<div class="flex items-center gap-2 flex-wrap">
			<label class="relative flex-1 min-w-[200px] max-w-sm">
				<span class="sr-only">{i18n.t('sync.lib.search')}</span>
				<Search class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" aria-hidden="true" />
				<input class="input pl-9" type="search" placeholder={i18n.t('sync.lib.search')} bind:value={query} />
			</label>
			<label class="text-meta flex items-center gap-1.5">
				{i18n.t('sync.lib.activeIn')}
				<select class="input py-1.5 w-auto" bind:value={activeIn}>
					<option value="any">{i18n.t('sync.lib.anyCli')}</option>
					{#each CLIS as cli (cli)}<option value={cli}>{CLI_SHORT[cli]}</option>{/each}
				</select>
			</label>
			<label class="text-meta flex items-center gap-1.5">
				{i18n.t('sync.lib.origin')}
				<select class="input py-1.5 w-auto" bind:value={origin}>
					<option value="any">{i18n.t('sync.lib.anyOrigin')}</option>
					<option value="global">{i18n.t('sync.lib.global')}</option>
					<option value="project">{i18n.t('sync.lib.project')}</option>
				</select>
			</label>
			<label class="text-meta flex items-center gap-1.5 cursor-pointer">
				<input type="checkbox" class="rounded border-gray-300 dark:border-gray-600" bind:checked={onlyProblems} />
				{i18n.t('sync.lib.onlyProblems')}
			</label>
			{#if hasFilters}
				<button class="btn btn-ghost text-xs px-2 py-1" onclick={clearFilters}>
					<X class="w-3 h-3 mr-1" aria-hidden="true" />{i18n.t('sync.lib.clear')}
				</button>
			{/if}
		</div>
	</div>

	<!-- List -->
	<div class="flex-1 overflow-y-auto bg-white dark:bg-gray-800/40">
		{#if loading && resources.length === 0}
			<Skeleton rows={8} />
		{:else if filtered.length === 0}
			<div class="flex flex-col items-center justify-center text-center py-16 px-6 gap-3">
				<p class="text-body font-medium">{resources.length === 0 ? i18n.t('sync.lib.emptyTitle') : i18n.t('sync.lib.noMatch')}</p>
				<p class="text-meta max-w-md">{resources.length === 0 ? i18n.t('sync.lib.emptyBody') : i18n.t('sync.lib.noMatchHint')}</p>
				{#if resources.length === 0 && onAdd}
					<button class="btn btn-primary" onclick={onAdd}>{i18n.t('sync.lib.addMcp')}</button>
				{:else if hasFilters}
					<button class="btn btn-secondary" onclick={clearFilters}>{i18n.t('sync.lib.clear')}</button>
				{/if}
			</div>
		{:else}
			<ul aria-label={i18n.t('sync.lib.listLabel')}>
				{#each filtered as r (r.uid)}
					{@const Icon = kindIcon[r.kind]}
					{@const problem = problemUids.has(r.uid) || r.literalSecretKeys.length > 0}
					<li>
					<div class="row-dense cursor-pointer" onclick={() => onOpen(r)}
						onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && e.target === e.currentTarget && (e.preventDefault(), onOpen(r))}
						tabindex="0" role="button" aria-label="{r.name} ({scopeLabel(r.scope)})">
						<Icon class="w-4 h-4 text-gray-400 shrink-0" aria-hidden="true" />
						<div class="flex-1 min-w-0">
							<p class="text-body truncate">
								<span class="font-medium text-gray-900 dark:text-white">{r.name}</span>
								{#if r.description}<span class="text-meta"> — {r.description}</span>{/if}
							</p>
						</div>
						{#if problem}
							<span class="chip border-amber-300 text-amber-700 dark:border-amber-700 dark:text-amber-300"
								title={r.literalSecretKeys.length ? i18n.t('sync.lib.literalSecret') : i18n.t('sync.lib.hasDivergence')}>
								<TriangleAlert class="w-3 h-3" aria-hidden="true" />{i18n.t('sync.lib.problem')}
							</span>
						{/if}
						<span class="chip border-gray-200 text-gray-600 dark:border-gray-700 dark:text-gray-300 max-w-[160px] truncate"
							title={r.scope.type === 'project' ? r.scope.path : i18n.t('sync.lib.globalHint')}>
							{scopeLabel(r.scope)}
						</span>
						<!-- svelte-ignore a11y_click_events_have_key_events -->
						<!-- svelte-ignore a11y_no_static_element_interactions -->
						<div onclick={(e) => e.stopPropagation()}>
							<CliChips targets={r.targets}
								disabled={syncStore.isBusy(`${r.scope.type === 'project' ? r.scope.path : 'global'}:${r.name}`) || syncStore.isBusy(`skill:${r.name}`)}
								onToggle={r.kind === 'agent' ? undefined : (cli, enabled) => toggle(r, cli, enabled)} />
						</div>
						<button class="p-1 rounded text-gray-400 hover:text-amber-500" onclick={(e) => toggleFavorite(r, e)}
							aria-pressed={r.isFavorite} aria-label={i18n.t('sync.lib.favorite')} title={i18n.t('sync.lib.favorite')}>
							<Star class="w-4 h-4 {r.isFavorite ? 'fill-amber-400 text-amber-500' : ''}" aria-hidden="true" />
						</button>
					</div>
					</li>
				{/each}
			</ul>
		{/if}
	</div>
</div>
