<script lang="ts">
	import { Search, X, Plus, Power, ArrowUpDown, ArrowUp, RefreshCw, TriangleAlert, Check, Minus, Plug, Link2, Ban, CircleCheck, CircleAlert, CircleX, CircleDashed } from 'lucide-svelte';
	import { syncStore } from '$lib/stores/syncStore.svelte';
	import { i18n } from '$lib/i18n';
	import type { Cli, LiveServer, ResourceKind, ResourceView, TargetChange, TargetState } from '$lib/types/sync';
	import { CLIS, CLI_LABEL, CLI_SHORT, isToggleable, scopeKey, scopeLabel } from '$lib/types/sync';
	import Skeleton from './Skeleton.svelte';

	type Props = {
		kind: ResourceKind;
		resources: ResourceView[];
		loading?: boolean;
		onOpen: (r: ResourceView) => void;
		onAdd?: () => void;
	};
	let { kind, resources, loading = false, onOpen, onAdd }: Props = $props();

	// ---------------------------------------------------------------- filters
	let query = $state('');
	let origin = $state<string>('any'); // any | global | <project path>
	let cliFilter = $state<string>('any'); // any | on:<cli> | off:<cli> | live:<cli>
	let onlyProblems = $state(false);
	type SortKey = 'name' | 'origin' | Cli;
	let sortKey = $state<SortKey>('origin');
	let sortAsc = $state(true);
	let selected = $state<Set<string>>(new Set());

	let rows = $derived(resources.filter((r) => r.kind === kind));
	let projects = $derived(
		[...new Set(rows.filter((r) => r.scope.type === 'project').map((r) => (r.scope as { path: string }).path))].sort()
	);

	// ---------------------------------------------------------------- live check
	type LiveMark = { tone: 'ok' | 'warn' | 'bad' | 'none'; text: string };

	function findLive(cli: Cli, name: string): LiveServer | undefined | null {
		const l = syncStore.live?.[cli];
		if (!l || !l.servers) return null; // not asked / failed
		return l.servers.find((s) => s.name === name || (cli === 'claude' && s.name.endsWith(':' + name)));
	}

	function liveMark(r: ResourceView, cli: Cli): LiveMark {
		if (kind !== 'mcp' || r.scope.type !== 'global') return { tone: 'none', text: '' };
		const found = findLive(cli, r.name);
		if (found === null) return { tone: 'none', text: '' };
		const st = r.targets[cli]?.state;
		const wanted = st === 'on' || st === 'providedByPlugin' || st === 'native';
		if (!found) {
			return wanted
				? { tone: 'bad', text: i18n.t('sync.live.notLoaded', { cli: CLI_LABEL[cli] }) }
				: { tone: 'none', text: '' };
		}
		if (!wanted && found.state !== 'disabled') return { tone: 'bad', text: i18n.t('sync.live.onButOff', { cli: CLI_LABEL[cli] }) };
		switch (found.state) {
			case 'connected':
				return { tone: 'ok', text: i18n.t('sync.live.connected', { cli: CLI_LABEL[cli] }) };
			case 'enabled':
				return { tone: 'ok', text: i18n.t('sync.live.loaded', { cli: CLI_LABEL[cli] }) };
			case 'disabled':
				return wanted
					? { tone: 'bad', text: i18n.t('sync.live.disabledInCli', { cli: CLI_LABEL[cli] }) }
					: { tone: 'none', text: '' };
			case 'needsAuth':
				return { tone: 'warn', text: i18n.t('sync.live.needsAuth', { cli: CLI_LABEL[cli] }) };
			default:
				return { tone: 'warn', text: i18n.t('sync.live.failed', { cli: CLI_LABEL[cli], detail: found.detail }) };
		}
	}

	/** Servers a CLI loads that the source does not have. */
	let outside = $derived.by(() => {
		if (kind !== 'mcp' || !syncStore.live) return [] as { cli: Cli; name: string; note: string }[];
		const known = new Set(rows.filter((r) => r.scope.type === 'global').map((r) => r.name));
		const out: { cli: Cli; name: string; note: string }[] = [];
		for (const cli of CLIS) {
			for (const s of syncStore.live[cli]?.servers ?? []) {
				const base = cli === 'claude' && s.name.startsWith('plugin:') ? s.name.split(':').pop()! : s.name;
				if (known.has(base) || s.state === 'disabled') continue;
				const note = s.name.startsWith('claude.ai ')
					? i18n.t('sync.live.connector')
					: s.name.startsWith('plugin:')
						? i18n.t('sync.live.plugin')
						: i18n.t('sync.live.onlyInCli');
				out.push({ cli, name: s.name, note });
			}
		}
		return out;
	});

	// ---------------------------------------------------------------- filtering / sorting
	function isOn(r: ResourceView, cli: Cli) {
		const s = r.targets[cli]?.state;
		return s === 'on' || s === 'native' || s === 'providedByPlugin';
	}

	let problemUids = $derived.by(() => {
		const set = new Set<string>();
		for (const d of syncStore.health?.divergences ?? []) {
			if (d.kind === 'agySkillsFolder') continue;
			set.add(`mcp:${scopeKey(d.scope)}:${d.name}`);
		}
		return set;
	});

	function hasProblem(r: ResourceView) {
		return problemUids.has(r.uid) || r.literalSecretKeys.length > 0 || CLIS.some((c) => liveMark(r, c).tone === 'bad');
	}

	let filtered = $derived.by(() => {
		const q = query.trim().toLowerCase();
		const out = rows.filter((r) => {
			if (origin === 'global' && r.scope.type !== 'global') return false;
			if (origin !== 'any' && origin !== 'global' && (r.scope.type !== 'project' || r.scope.path !== origin)) return false;
			if (cliFilter !== 'any') {
				const [mode, cli] = cliFilter.split(':') as ['on' | 'off' | 'live', Cli];
				if (mode === 'on' && !isOn(r, cli)) return false;
				if (mode === 'off' && isOn(r, cli)) return false;
				if (mode === 'live' && liveMark(r, cli).tone !== 'bad') return false;
			}
			if (onlyProblems && !hasProblem(r)) return false;
			if (q && !`${r.name} ${r.description ?? ''} ${scopeLabel(r.scope)}`.toLowerCase().includes(q)) return false;
			return true;
		});
		const dir = sortAsc ? 1 : -1;
		const originRank = (r: ResourceView) => (r.scope.type === 'global' ? '0' : '1' + scopeLabel(r.scope));
		return out.sort((a, b) => {
			let c = 0;
			if (sortKey === 'name') c = a.name.localeCompare(b.name);
			else if (sortKey === 'origin') c = originRank(a).localeCompare(originRank(b)) || a.name.localeCompare(b.name);
			else c = Number(isOn(b, sortKey)) - Number(isOn(a, sortKey)) || a.name.localeCompare(b.name);
			return c * dir;
		});
	});

	function sortBy(k: SortKey) {
		if (sortKey === k) sortAsc = !sortAsc;
		else {
			sortKey = k;
			sortAsc = true;
		}
	}

	let hasFilters = $derived(query !== '' || origin !== 'any' || cliFilter !== 'any' || onlyProblems);
	function clearFilters() {
		query = '';
		origin = 'any';
		cliFilter = 'any';
		onlyProblems = false;
	}

	// ---------------------------------------------------------------- switching
	const projectOf = (r: ResourceView) => (r.scope.type === 'project' ? r.scope.path : null);
	const busyKey = (r: ResourceView) => (kind === 'skill' ? `skill:${r.name}` : `${projectOf(r) ?? 'global'}:${r.name}`);
	let bulkBusy = $derived(syncStore.isBusy('bulk'));

	function canSwitch(r: ResourceView, cli: Cli) {
		return kind !== 'agent' && isToggleable(r.targets[cli]);
	}

	async function apply(changes: TargetChange[]) {
		if (!changes.length) return;
		if (kind === 'skill') {
			for (const c of changes) await syncStore.setSkillTarget(c.name, c.cli, c.enabled);
		} else if (changes.length === 1) {
			const c = changes[0];
			await syncStore.setTarget(c.name, c.cli, c.enabled, c.project);
		} else {
			await syncStore.setTargets(changes);
		}
	}

	function changesFor(list: ResourceView[], clis: Cli[], enabled: boolean): TargetChange[] {
		const out: TargetChange[] = [];
		for (const r of list)
			for (const cli of clis)
				if (canSwitch(r, cli) && (r.targets[cli]?.state === 'on') !== enabled)
					out.push({ project: projectOf(r), name: r.name, cli, enabled });
		return out;
	}

	function rowAllOn(r: ResourceView) {
		return CLIS.filter((c) => canSwitch(r, c)).every((c) => r.targets[c]?.state === 'on');
	}

	function columnState(cli: Cli): 'all' | 'some' | 'none' | 'na' {
		const sw = filtered.filter((r) => canSwitch(r, cli));
		if (!sw.length) return 'na';
		const on = sw.filter((r) => r.targets[cli]?.state === 'on').length;
		return on === sw.length ? 'all' : on === 0 ? 'none' : 'some';
	}

	function toggleColumn(cli: Cli) {
		const st = columnState(cli);
		if (st === 'na') return;
		apply(changesFor(filtered, [cli], st !== 'all'));
	}

	// ---------------------------------------------------------------- selection
	let selectedRows = $derived(filtered.filter((r) => selected.has(r.uid)));
	let allSelected = $derived(filtered.length > 0 && filtered.every((r) => selected.has(r.uid)));

	function toggleSelect(r: ResourceView) {
		const next = new Set(selected);
		if (next.has(r.uid)) next.delete(r.uid);
		else next.add(r.uid);
		selected = next;
	}
	function toggleSelectAll() {
		selected = allSelected ? new Set() : new Set(filtered.map((r) => r.uid));
	}
	async function bulk(clis: Cli[], enabled: boolean) {
		await apply(changesFor(selectedRows, clis, enabled));
	}

	// ---------------------------------------------------------------- view helpers
	const stateIcon = { on: Check, off: Minus, providedByPlugin: Plug, native: Link2, unsupported: Ban } as const;
	function cellTitle(r: ResourceView, cli: Cli, st: TargetState | undefined, live: LiveMark) {
		const name = CLI_LABEL[cli];
		const base =
			!st || st.state === 'off'
				? i18n.t('sync.chip.off', { cli: name })
				: st.state === 'on'
					? i18n.t('sync.chip.on', { cli: name })
					: st.state === 'providedByPlugin'
						? i18n.t('sync.chip.plugin', { cli: name })
						: st.state === 'native'
							? i18n.t('sync.chip.native', { cli: name })
							: st.reason;
		return live.text ? `${base}\n${live.text}` : base;
	}
	const liveIcon = { ok: CircleCheck, warn: CircleAlert, bad: CircleX, none: CircleDashed };
	const liveColor = { ok: 'text-emerald-500', warn: 'text-amber-500', bad: 'text-red-500', none: 'text-transparent' };
	const grid = 'grid grid-cols-[28px_minmax(180px,1fr)_minmax(110px,180px)_repeat(3,76px)_40px] items-center gap-x-2';
</script>

<div class="flex flex-col h-full">
	<!-- Toolbar -->
	<div class="px-6 py-3 border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 flex items-center gap-2 flex-wrap">
		<label class="relative w-56">
			<span class="sr-only">{i18n.t('sync.lib.search')}</span>
			<Search class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" aria-hidden="true" />
			<input class="input input-sm pl-9" type="search" placeholder={i18n.t('sync.lib.search')} bind:value={query} />
		</label>
		<label class="text-meta flex items-center gap-1.5">
			{i18n.t('sync.lib.origin')}
			<select class="input input-sm w-auto max-w-[200px]" bind:value={origin}>
				<option value="any">{i18n.t('sync.lib.anyOrigin')}</option>
				<option value="global">{i18n.t('sync.lib.global')}</option>
				{#each projects as p (p)}<option value={p}>{scopeLabel({ type: 'project', path: p })}</option>{/each}
			</select>
		</label>
		<label class="text-meta flex items-center gap-1.5">
			{i18n.t('sync.matrix.show')}
			<select class="input input-sm w-auto" bind:value={cliFilter}>
				<option value="any">{i18n.t('sync.matrix.showAll')}</option>
				{#each CLIS as cli (cli)}<option value="on:{cli}">{i18n.t('sync.matrix.onIn', { cli: CLI_SHORT[cli] })}</option>{/each}
				{#each CLIS as cli (cli)}<option value="off:{cli}">{i18n.t('sync.matrix.offIn', { cli: CLI_SHORT[cli] })}</option>{/each}
				{#if kind === 'mcp'}{#each CLIS as cli (cli)}<option value="live:{cli}">{i18n.t('sync.matrix.notLoadedIn', { cli: CLI_SHORT[cli] })}</option>{/each}{/if}
			</select>
		</label>
		<label class="text-meta flex items-center gap-1.5 cursor-pointer">
			<input type="checkbox" class="rounded border-gray-300 dark:border-gray-600" bind:checked={onlyProblems} />
			{i18n.t('sync.lib.onlyProblems')}
		</label>
		{#if hasFilters}
			<button class="btn btn-ghost text-xs px-2 py-1" onclick={clearFilters}><X class="w-3 h-3 mr-1" aria-hidden="true" />{i18n.t('sync.lib.clear')}</button>
		{/if}
		<div class="flex-1"></div>
		{#if kind === 'mcp'}
			<button class="btn btn-ghost text-xs" onclick={() => syncStore.loadLive()} disabled={syncStore.isLoadingLive} title={i18n.t('sync.live.checkHint')}>
				<RefreshCw class="w-3.5 h-3.5 mr-1.5 {syncStore.isLoadingLive ? 'animate-spin' : ''}" aria-hidden="true" />
				{syncStore.isLoadingLive ? i18n.t('sync.live.checking') : i18n.t('sync.live.check')}
			</button>
		{/if}
		{#if onAdd}
			<button class="btn btn-primary" onclick={onAdd}><Plus class="w-4 h-4 mr-1.5" aria-hidden="true" />{i18n.t('sync.lib.addMcp')}</button>
		{/if}
	</div>

	<!-- Bulk bar -->
	{#if selectedRows.length}
		<div class="px-6 py-2 border-b border-primary-200 dark:border-primary-800 bg-primary-50 dark:bg-primary-950/40 flex items-center gap-2 flex-wrap text-sm" role="region" aria-label={i18n.t('sync.matrix.bulkLabel')}>
			<span class="font-medium">{i18n.t('sync.matrix.selected', { n: selectedRows.length })}</span>
			<span class="text-meta ml-2">{i18n.t('sync.matrix.turnOnIn')}</span>
			{#each CLIS as cli (cli)}<button class="btn btn-secondary text-xs px-2 py-1" disabled={bulkBusy} onclick={() => bulk([cli], true)}>{CLI_SHORT[cli]}</button>{/each}
			<button class="btn btn-primary text-xs px-2 py-1" disabled={bulkBusy} onclick={() => bulk(CLIS, true)}>{i18n.t('sync.matrix.allThree')}</button>
			<span class="text-meta ml-2">{i18n.t('sync.matrix.turnOffIn')}</span>
			{#each CLIS as cli (cli)}<button class="btn btn-secondary text-xs px-2 py-1" disabled={bulkBusy} onclick={() => bulk([cli], false)}>{CLI_SHORT[cli]}</button>{/each}
			<button class="btn btn-secondary text-xs px-2 py-1" disabled={bulkBusy} onclick={() => bulk(CLIS, false)}>{i18n.t('sync.matrix.allThree')}</button>
			<div class="flex-1"></div>
			<button class="btn btn-ghost text-xs px-2 py-1" onclick={() => (selected = new Set())}>{i18n.t('sync.matrix.clearSelection')}</button>
		</div>
	{/if}

	<div class="flex-1 overflow-auto bg-white dark:bg-gray-800/40">
		{#if loading && rows.length === 0}
			<Skeleton rows={8} />
		{:else}
			<div role="table" aria-label={i18n.t('sync.lib.listLabel')} class="min-w-[640px]">
				<!-- Header -->
				<div role="row" class="{grid} px-6 py-2 text-meta font-medium border-b border-gray-200 dark:border-gray-700 sticky top-0 bg-white dark:bg-gray-800 z-10">
					<span role="columnheader"><input type="checkbox" class="rounded border-gray-300 dark:border-gray-600" checked={allSelected} onchange={toggleSelectAll} aria-label={i18n.t('sync.matrix.selectAll')} /></span>
					<button role="columnheader" class="text-left flex items-center gap-1 hover:text-gray-900 dark:hover:text-white" onclick={() => sortBy('name')} aria-sort={sortKey === 'name' ? (sortAsc ? 'ascending' : 'descending') : 'none'}>
						{i18n.t('sync.matrix.name')} ({filtered.length})
						{#if sortKey === 'name'}<ArrowUp class="w-3 h-3 {sortAsc ? '' : 'rotate-180'}" aria-hidden="true" />{:else}<ArrowUpDown class="w-3 h-3 opacity-40" aria-hidden="true" />{/if}
					</button>
					<button role="columnheader" class="text-left flex items-center gap-1 hover:text-gray-900 dark:hover:text-white" onclick={() => sortBy('origin')} aria-sort={sortKey === 'origin' ? (sortAsc ? 'ascending' : 'descending') : 'none'}>
						{i18n.t('sync.lib.origin')}
						{#if sortKey === 'origin'}<ArrowUp class="w-3 h-3 {sortAsc ? '' : 'rotate-180'}" aria-hidden="true" />{:else}<ArrowUpDown class="w-3 h-3 opacity-40" aria-hidden="true" />{/if}
					</button>
					{#each CLIS as cli (cli)}
						<button role="columnheader" class="flex items-center justify-center gap-1 hover:text-gray-900 dark:hover:text-white" onclick={() => sortBy(cli)} title={i18n.t('sync.matrix.sortByCli', { cli: CLI_LABEL[cli] })} aria-sort={sortKey === cli ? (sortAsc ? 'ascending' : 'descending') : 'none'}>
							{CLI_SHORT[cli]}
							{#if sortKey === cli}<ArrowUp class="w-3 h-3 {sortAsc ? '' : 'rotate-180'}" aria-hidden="true" />{:else}<ArrowUpDown class="w-3 h-3 opacity-40" aria-hidden="true" />{/if}
						</button>
					{/each}
					<span role="columnheader" class="sr-only">{i18n.t('sync.matrix.allThree')}</span>
				</div>
				<!-- Column switches -->
				{#if filtered.length}
					<div role="row" class="{grid} px-6 py-1.5 border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900/40 text-meta">
						<span></span>
						<span>{i18n.t('sync.matrix.wholeColumn')}</span>
						<span></span>
						{#each CLIS as cli (cli)}
							{@const st = columnState(cli)}
							<span class="flex justify-center">
								<input type="checkbox" class="rounded border-gray-300 dark:border-gray-600 disabled:opacity-30"
									checked={st === 'all'} indeterminate={st === 'some'} disabled={st === 'na' || bulkBusy}
									onchange={() => toggleColumn(cli)}
									aria-label={st === 'all' ? i18n.t('sync.matrix.columnOff', { cli: CLI_LABEL[cli], n: filtered.length }) : i18n.t('sync.matrix.columnOn', { cli: CLI_LABEL[cli], n: filtered.length })}
									title={st === 'all' ? i18n.t('sync.matrix.columnOff', { cli: CLI_LABEL[cli], n: filtered.length }) : i18n.t('sync.matrix.columnOn', { cli: CLI_LABEL[cli], n: filtered.length })} />
							</span>
						{/each}
						<span></span>
					</div>
				{/if}

				{#if filtered.length === 0}
					<div class="flex flex-col items-center justify-center text-center py-16 px-6 gap-3">
						<p class="text-body font-medium">{rows.length === 0 ? i18n.t('sync.lib.emptyTitle') : i18n.t('sync.lib.noMatch')}</p>
						{#if hasFilters}<button class="btn btn-secondary" onclick={clearFilters}>{i18n.t('sync.lib.clear')}</button>{/if}
					</div>
				{/if}

				{#each filtered as r, i (r.uid)}
					{@const problem = hasProblem(r)}
					{#if sortKey === 'origin' && (i === 0 || scopeLabel(filtered[i - 1].scope) !== scopeLabel(r.scope))}
						{@const groupCount = filtered.filter((x) => scopeLabel(x.scope) === scopeLabel(r.scope)).length}
						<div role="row" class="px-6 pt-3 pb-1 text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400 border-b border-gray-100 dark:border-gray-700/60 bg-gray-50/60 dark:bg-gray-900/30">
							<span role="cell">{r.scope.type === 'global' ? i18n.t('sync.matrix.groupGlobal') : scopeLabel(r.scope)} <span class="font-normal normal-case">({groupCount}){r.scope.type === 'project' ? ' · ' + r.scope.path : ''}</span></span>
						</div>
					{/if}
					<div role="row" class="{grid} px-6 py-1.5 border-b border-gray-100 dark:border-gray-700/60 hover:bg-gray-50 dark:hover:bg-gray-700/30 {selected.has(r.uid) ? 'bg-primary-50/60 dark:bg-primary-950/30' : ''}">
						<span role="cell"><input type="checkbox" class="rounded border-gray-300 dark:border-gray-600" checked={selected.has(r.uid)} onchange={() => toggleSelect(r)} aria-label={i18n.t('sync.matrix.selectRow', { name: r.name })} /></span>
						<span role="cell" class="min-w-0">
							<button class="text-left w-full min-w-0 flex items-center gap-1.5 group" onclick={() => onOpen(r)}>
								<span class="font-medium text-sm text-gray-900 dark:text-white truncate group-hover:underline">{r.name}</span>
								{#if problem}<TriangleAlert class="w-3.5 h-3.5 shrink-0 text-amber-500" aria-label={i18n.t('sync.lib.problem')} />{/if}
							</button>
							{#if r.description}<span class="block text-meta truncate">{r.description}</span>{/if}
						</span>
						<span role="cell" class="text-meta truncate" title={r.scope.type === 'project' ? r.scope.path : i18n.t('sync.lib.globalHint')}>{sortKey === 'origin' ? '' : scopeLabel(r.scope)}</span>
						{#each CLIS as cli (cli)}
							{@const st = r.targets[cli]}
							{@const k = st?.state ?? 'off'}
							{@const Icon = stateIcon[k]}
							{@const live = liveMark(r, cli)}
							{@const LiveIcon = liveIcon[live.tone]}
							{@const sw = canSwitch(r, cli) && !syncStore.isBusy(busyKey(r)) && !bulkBusy}
							<span role="cell" class="flex items-center justify-center gap-1">
								<button type="button" disabled={!sw}
									class="w-7 h-7 rounded-md border flex items-center justify-center transition-colors
										{k === 'on' ? 'bg-primary-600 border-primary-600 text-white hover:bg-primary-700' : ''}
										{k === 'off' ? 'border-gray-300 dark:border-gray-600 text-gray-400 hover:border-primary-400' : ''}
										{k === 'providedByPlugin' || k === 'native' ? 'border-dashed border-primary-300 dark:border-primary-700 text-primary-600 dark:text-primary-300' : ''}
										{k === 'unsupported' ? 'border-gray-200 dark:border-gray-700 text-gray-300 dark:text-gray-600' : ''}
										{sw ? 'cursor-pointer' : 'cursor-default'}"
									aria-pressed={k === 'on'}
									aria-label="{r.name} · {cellTitle(r, cli, st, live)}"
									title={cellTitle(r, cli, st, live)}
									onclick={() => sw && apply([{ project: projectOf(r), name: r.name, cli, enabled: k !== 'on' }])}>
									<Icon class="w-3.5 h-3.5" aria-hidden="true" />
								</button>
								<LiveIcon class="w-3.5 h-3.5 {liveColor[live.tone]}" aria-hidden="true" />
							</span>
						{/each}
						<span role="cell" class="flex justify-center">
							{#if CLIS.some((c) => canSwitch(r, c))}
								<button class="btn btn-ghost p-1.5" disabled={syncStore.isBusy(busyKey(r)) || bulkBusy}
									onclick={() => apply(changesFor([r], CLIS, !rowAllOn(r)))}
									title={rowAllOn(r) ? i18n.t('sync.matrix.rowOff') : i18n.t('sync.matrix.rowOn')}
									aria-label="{r.name}: {rowAllOn(r) ? i18n.t('sync.matrix.rowOff') : i18n.t('sync.matrix.rowOn')}">
									<Power class="w-4 h-4 {rowAllOn(r) ? 'text-primary-600 dark:text-primary-300' : 'text-gray-400'}" aria-hidden="true" />
								</button>
							{/if}
						</span>
					</div>
				{/each}
			</div>

			<!-- Legend -->
			<div class="px-6 py-3 text-meta flex items-center gap-x-4 gap-y-1 flex-wrap border-b border-gray-100 dark:border-gray-700/60">
				<span class="flex items-center gap-1"><Check class="w-3 h-3" aria-hidden="true" />{i18n.t('sync.matrix.legendOn')}</span>
				<span class="flex items-center gap-1"><Plug class="w-3 h-3" aria-hidden="true" />{i18n.t('sync.matrix.legendPlugin')}</span>
				<span class="flex items-center gap-1"><Link2 class="w-3 h-3" aria-hidden="true" />{i18n.t('sync.matrix.legendNative')}</span>
				{#if kind === 'mcp'}
					<span class="flex items-center gap-1"><CircleCheck class="w-3 h-3 text-emerald-500" aria-hidden="true" />{i18n.t('sync.matrix.legendLoaded')}</span>
					<span class="flex items-center gap-1"><CircleAlert class="w-3 h-3 text-amber-500" aria-hidden="true" />{i18n.t('sync.matrix.legendWarn')}</span>
					<span class="flex items-center gap-1"><CircleX class="w-3 h-3 text-red-500" aria-hidden="true" />{i18n.t('sync.matrix.legendBad')}</span>
					<span>{syncStore.live ? i18n.t('sync.matrix.liveGlobalOnly') : i18n.t('sync.matrix.liveNotChecked')}</span>
				{/if}
			</div>

			<!-- Loaded by a CLI but not in the source -->
			{#if outside.length}
				<section class="px-6 py-4 space-y-2">
					<h3 class="text-body font-semibold">{i18n.t('sync.live.outsideTitle', { n: outside.length })}</h3>
					<p class="text-meta">{i18n.t('sync.live.outsideHint')}</p>
					<ul class="grid grid-cols-1 md:grid-cols-3 gap-2">
						{#each CLIS as cli (cli)}
							{@const items = outside.filter((o) => o.cli === cli)}
							<li class="rounded-lg border border-gray-200 dark:border-gray-700 p-3">
								<p class="text-body font-medium mb-1">{CLI_LABEL[cli]} <span class="text-meta">({items.length})</span></p>
								{#each items as o (o.name)}
									<p class="text-sm truncate" title={o.note}>{o.name} <span class="text-meta">· {o.note}</span></p>
								{:else}
									<p class="text-meta">{i18n.t('sync.live.none')}</p>
								{/each}
							</li>
						{/each}
					</ul>
				</section>
			{/if}
		{/if}
	</div>
</div>
