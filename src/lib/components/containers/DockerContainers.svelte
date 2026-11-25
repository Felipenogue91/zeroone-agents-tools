<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { Play, Square, RotateCw, ScrollText, Search, X } from 'lucide-svelte';
	import { notifications } from '$lib/stores';
	import { i18n } from '$lib/i18n';
	import SidePanel from '$lib/components/sync/SidePanel.svelte';
	import Skeleton from '$lib/components/sync/Skeleton.svelte';

	type DockerContainer = {
		dockerId: string;
		name: string;
		image: string;
		state: string;
		status: string;
		ports: string[];
		composeProject: string | null;
		composeService: string | null;
		workingDir: string | null;
		created: number;
	};
	type LogLine = { timestamp?: string | null; stream?: string; message: string };

	let list = $state<DockerContainer[] | null>(null);
	let error = $state<string | null>(null);
	let query = $state('');
	let onlyRunning = $state(false);
	let busy = $state<Set<string>>(new Set());
	let logs = $state<{ c: DockerContainer; lines: LogLine[] | null } | null>(null);
	let timer: ReturnType<typeof setInterval> | null = null;

	async function load() {
		try {
			list = await invoke<DockerContainer[]>('list_docker_containers');
			error = null;
		} catch (e) {
			error = String(e);
		}
	}

	onMount(() => {
		load();
		timer = setInterval(load, 5000);
	});
	onDestroy(() => timer && clearInterval(timer));

	let filtered = $derived.by(() => {
		const q = query.trim().toLowerCase();
		return (list ?? []).filter(
			(c) =>
				(!onlyRunning || c.state === 'running') &&
				(!q || `${c.name} ${c.image} ${c.composeProject ?? ''}`.toLowerCase().includes(q))
		);
	});

	let groups = $derived.by(() => {
		const m = new Map<string, DockerContainer[]>();
		for (const c of filtered) {
			const k = c.composeProject ?? '';
			m.set(k, [...(m.get(k) ?? []), c]);
		}
		return [...m.entries()].map(([project, items]) => ({
			project,
			items,
			running: items.filter((c) => c.state === 'running').length,
			workingDir: items.find((c) => c.workingDir)?.workingDir ?? null
		}));
	});

	let counts = $derived({
		total: list?.length ?? 0,
		running: (list ?? []).filter((c) => c.state === 'running').length
	});

	async function act(c: DockerContainer, action: 'start' | 'stop' | 'restart') {
		busy = new Set([...busy, c.dockerId]);
		try {
			await invoke('docker_container_action', { dockerId: c.dockerId, action });
			await load();
		} catch (e) {
			notifications.add('error', i18n.t('docker.actionFailed', { name: c.name }), { detail: String(e), duration: 8000 });
		} finally {
			const n = new Set(busy);
			n.delete(c.dockerId);
			busy = n;
		}
	}

	async function actGroup(items: DockerContainer[], action: 'start' | 'stop') {
		for (const c of items) {
			if (action === 'start' ? c.state !== 'running' : c.state === 'running') await act(c, action);
		}
	}

	async function openLogs(c: DockerContainer) {
		logs = { c, lines: null };
		try {
			const lines = await invoke<LogLine[]>('docker_container_logs', { dockerId: c.dockerId, tail: 200 });
			if (logs?.c.dockerId === c.dockerId) logs = { c, lines };
		} catch (e) {
			notifications.add('error', i18n.t('docker.logsFailed'), { detail: String(e) });
			logs = null;
		}
	}

	const stateStyle: Record<string, string> = {
		running: 'bg-emerald-500',
		restarting: 'bg-amber-500',
		paused: 'bg-amber-500',
		exited: 'bg-gray-400 dark:bg-gray-500',
		created: 'bg-gray-300 dark:bg-gray-600',
		dead: 'bg-red-500'
	};
</script>

<section class="space-y-3" aria-labelledby="docker-title">
	<div class="flex items-center gap-3 flex-wrap">
		<h2 id="docker-title" class="text-title flex-1">
			{i18n.t('docker.title')}
			{#if list}<span class="text-meta font-normal">· {i18n.t('docker.counts', { running: counts.running, total: counts.total })}</span>{/if}
		</h2>
		<label class="relative w-56">
			<span class="sr-only">{i18n.t('docker.search')}</span>
			<Search class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" aria-hidden="true" />
			<input class="input input-sm pl-9" type="search" placeholder={i18n.t('docker.search')} bind:value={query} />
		</label>
		<label class="text-meta flex items-center gap-1.5 cursor-pointer">
			<input type="checkbox" class="rounded border-gray-300 dark:border-gray-600" bind:checked={onlyRunning} />{i18n.t('docker.onlyRunning')}
		</label>
	</div>

	{#if error}
		<div class="card text-body">
			<p class="font-medium">{i18n.t('docker.unavailable')}</p>
			<p class="text-meta break-all mt-1">{error}</p>
		</div>
	{:else if !list}
		<div class="card"><Skeleton rows={4} /></div>
	{:else if filtered.length === 0}
		<div class="card text-center text-meta py-8">
			{list.length === 0 ? i18n.t('docker.empty') : i18n.t('docker.noMatch')}
			{#if query || onlyRunning}
				<button class="btn btn-ghost text-xs ml-2" onclick={() => { query = ''; onlyRunning = false; }}><X class="w-3 h-3 mr-1" aria-hidden="true" />{i18n.t('sync.lib.clear')}</button>
			{/if}
		</div>
	{:else}
		{#each groups as g (g.project)}
			<section class="card p-0 overflow-hidden">
				<header class="flex items-center gap-3 px-4 py-2.5 border-b border-gray-200 dark:border-gray-700">
					<div class="flex-1 min-w-0">
						<h3 class="text-body font-semibold">{g.project || i18n.t('docker.standalone')} <span class="text-meta font-normal">· {i18n.t('docker.groupRunning', { running: g.running, total: g.items.length })}</span></h3>
						{#if g.workingDir}<p class="text-meta truncate" title={g.workingDir}>{g.workingDir}</p>{/if}
					</div>
					{#if g.project && g.items.length > 1}
						{#if g.running < g.items.length}
							<button data-tip={i18n.t('tip.docker.startAll')} class="btn btn-ghost text-xs px-2 py-1" onclick={() => actGroup(g.items, 'start')}><Play class="w-3.5 h-3.5 mr-1" aria-hidden="true" />{i18n.t('docker.startAll')}</button>
						{/if}
						{#if g.running > 0}
							<button data-tip={i18n.t('tip.docker.stopAll')} class="btn btn-ghost text-xs px-2 py-1" onclick={() => actGroup(g.items, 'stop')}><Square class="w-3.5 h-3.5 mr-1" aria-hidden="true" />{i18n.t('docker.stopAll')}</button>
						{/if}
					{/if}
				</header>
				<ul>
					{#each g.items as c (c.dockerId)}
						{@const isBusy = busy.has(c.dockerId)}
						<li class="row-dense">
							<span class="w-2.5 h-2.5 rounded-full shrink-0 {stateStyle[c.state] ?? 'bg-gray-400'}" aria-hidden="true"></span>
							<div class="flex-1 min-w-0">
								<p class="text-body truncate"><span class="font-medium text-gray-900 dark:text-white">{c.composeService ?? c.name}</span>
									<span class="text-meta"> · {c.image}</span></p>
								<p class="text-meta truncate">
									<span class="sr-only">{i18n.t('docker.state')}: </span>{c.status}
									{#if c.ports.length} · {c.ports.join(', ')}{/if}
								</p>
							</div>
							<div class="flex items-center gap-1 shrink-0">
								{#if c.state === 'running'}
									<button data-tip={i18n.t('tip.docker.restart')} class="btn btn-ghost p-1.5" disabled={isBusy} onclick={() => act(c, 'restart')} title={i18n.t('docker.restart')} aria-label="{i18n.t('docker.restart')} {c.name}"><RotateCw class="w-4 h-4 {isBusy ? 'animate-spin' : ''}" aria-hidden="true" /></button>
									<button data-tip={i18n.t('tip.docker.stop')} class="btn btn-ghost p-1.5" disabled={isBusy} onclick={() => act(c, 'stop')} title={i18n.t('docker.stop')} aria-label="{i18n.t('docker.stop')} {c.name}"><Square class="w-4 h-4" aria-hidden="true" /></button>
								{:else}
									<button data-tip={i18n.t('tip.docker.start')} class="btn btn-ghost p-1.5" disabled={isBusy} onclick={() => act(c, 'start')} title={i18n.t('docker.start')} aria-label="{i18n.t('docker.start')} {c.name}"><Play class="w-4 h-4" aria-hidden="true" /></button>
								{/if}
								<button data-tip={i18n.t('tip.docker.logs')} class="btn btn-ghost p-1.5" onclick={() => openLogs(c)} title={i18n.t('docker.logs')} aria-label="{i18n.t('docker.logs')} {c.name}"><ScrollText class="w-4 h-4" aria-hidden="true" /></button>
							</div>
						</li>
					{/each}
				</ul>
			</section>
		{/each}
	{/if}
</section>

<SidePanel open={!!logs} title={logs ? i18n.t('docker.logsOf', { name: logs.c.name }) : ''} subtitle={logs?.c.status} onClose={() => (logs = null)}>
	{#if logs && !logs.lines}
		<Skeleton rows={8} />
	{:else if logs?.lines}
		<pre class="font-mono text-xs whitespace-pre-wrap break-all bg-gray-50 dark:bg-gray-900 rounded-lg p-3 max-h-[70vh] overflow-auto">{logs.lines.map((l) => l.message).join('\n') || i18n.t('docker.noLogs')}</pre>
	{/if}
	{#snippet footer()}
		<button data-tip={i18n.t('tip.docker.reloadLogs')} class="btn btn-ghost" onclick={() => logs && openLogs(logs.c)}>{i18n.t('docker.reloadLogs')}</button>
		<button class="btn btn-primary" onclick={() => (logs = null)}>{i18n.t('docker.close')}</button>
	{/snippet}
</SidePanel>
