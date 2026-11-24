<script lang="ts">
	import { onMount } from 'svelte';
	import { TriangleAlert, CircleCheck, FileCode2, RefreshCcw, ChevronDown } from 'lucide-svelte';
	import { Header } from '$lib/components/layout';
	import { syncStore } from '$lib/stores/syncStore.svelte';
	import { notifications } from '$lib/stores';
	import { i18n } from '$lib/i18n';
	import CliChip from '$lib/components/sync/CliChip.svelte';
	import DiffView from '$lib/components/sync/DiffView.svelte';
	import DivergenceList from '$lib/components/sync/DivergenceList.svelte';
	import SidePanel from '$lib/components/sync/SidePanel.svelte';
	import Skeleton from '$lib/components/sync/Skeleton.svelte';
	import type { ChangeView, Cli, Divergence } from '$lib/types/sync';
	import { CLIS, CLI_LABEL } from '$lib/types/sync';

	const files: Record<Cli, { path: string; key: string }[]> = {
		claude: [
			{ path: '~/.claude.json', key: 'sync.clis.claudeUser' },
			{ path: '<projeto>/.mcp.json', key: 'sync.clis.claudeProject' },
			{ path: '~/.claude/skills/', key: 'sync.clis.claudeSkills' }
		],
		codex: [
			{ path: '~/.codex/config.toml', key: 'sync.clis.codexGlobal' },
			{ path: '<projeto>/.codex/config.toml', key: 'sync.clis.codexProject' },
			{ path: '~/.agents/skills/', key: 'sync.clis.codexSkills' }
		],
		agy: [
			{ path: '~/.gemini/config/mcp_config.json', key: 'sync.clis.agyGlobal' },
			{ path: '<pasta>/.agents/mcp_config.json', key: 'sync.clis.agyProject' },
			{ path: '~/.gemini/config/skills.json', key: 'sync.clis.agySkills' }
		]
	};

	let panel = $state<{ cli: Cli; changes: ChangeView[]; loading: boolean } | null>(null);

	onMount(() => {
		if (!syncStore.health) syncStore.loadHealth();
		if (syncStore.resources.length === 0) syncStore.loadResources();
	});

	function reapplyDiv(cli: Cli): Divergence {
		return { kind: 'drift', cli, scope: { type: 'global' }, name: '', filePath: '', message: '', actions: ['reapply'] };
	}

	/** What "Igualar à fonte" will do, from the divergences of that CLI. */
	function summary(cli: Cli) {
		const divs = syncStore.health?.divergences.filter((d) => d.cli === cli) ?? [];
		return {
			divs,
			add: divs.filter((d) => d.kind === 'missingInCli').length,
			update: divs.filter((d) => d.kind === 'drift').length,
			remove: divs.filter((d) => d.kind === 'onlyInCli'),
			other: divs.filter((d) => d.kind === 'claudeLocalScope' || d.kind === 'agySkillsFolder').length
		};
	}

	function active(cli: Cli) {
		return syncStore.resources.filter((r) => {
			const s = r.targets[cli]?.state;
			return s === 'on' || s === 'native' || s === 'providedByPlugin';
		}).length;
	}

	async function openReapply(cli: Cli) {
		panel = { cli, changes: [], loading: true };
		try {
			const changes = await syncStore.previewAction(reapplyDiv(cli), 'reapply');
			if (panel) panel = { cli, changes, loading: false };
		} catch (e) {
			notifications.add('error', i18n.t('sync.previewFailed'), { detail: String(e) });
			panel = null;
		}
	}

	async function confirm() {
		if (!panel) return;
		const cli = panel.cli;
		panel = null;
		await syncStore.applyAction(reapplyDiv(cli), 'reapply');
	}
</script>

<Header title={i18n.t('sync.clis.title')} subtitle={i18n.t('sync.clis.subtitle')} />

<div class="flex-1 overflow-y-auto">
	<div class="max-w-5xl mx-auto px-6 py-6 space-y-4">
		{#if !syncStore.health}
			<div class="card"><Skeleton rows={3} /></div>
		{:else}
			{#each CLIS as cli (cli)}
				{@const v = syncStore.health.cliVersions.find((x) => x.cli === cli)}
				{@const sum = summary(cli)}
				<section class="card space-y-3" aria-labelledby="cli-{cli}" id="cli-{cli}">
					<div class="flex items-center gap-3 flex-wrap">
						<CliChip {cli} label />
						<h2 id="cli-{cli}-title" class="text-title flex-1">{CLI_LABEL[cli]}</h2>
						{#if sum.divs.length}
							<span class="chip border-amber-300 text-amber-700 dark:border-amber-700 dark:text-amber-300"><TriangleAlert class="w-3 h-3" aria-hidden="true" />{i18n.t('sync.home.cliPending', { n: sum.divs.length })}</span>
							<button class="btn btn-secondary" onclick={() => openReapply(cli)} title={i18n.t('sync.clis.matchAllHint')}>
								<RefreshCcw class="w-4 h-4 mr-1.5" aria-hidden="true" />{i18n.t('sync.clis.matchAll')}
							</button>
						{:else}
							<span class="chip border-emerald-300 text-emerald-700 dark:border-emerald-800 dark:text-emerald-300"><CircleCheck class="w-3 h-3" aria-hidden="true" />{i18n.t('sync.home.cliOk')}</span>
						{/if}
					</div>
					<p class="text-body">
						{#if v?.installed}
							{i18n.t('sync.clis.version', { v: v.installed, tested: v.tested })}
						{:else}
							{i18n.t('sync.cli.notInstalled')}
						{/if}
						· {i18n.t('sync.clis.active', { n: active(cli) })}
					</p>
					{#if v?.message}
						<p class="text-body text-amber-700 dark:text-amber-300 flex items-center gap-1.5"><TriangleAlert class="w-4 h-4" aria-hidden="true" />{v.message}</p>
					{/if}
					<ul class="grid grid-cols-1 md:grid-cols-3 gap-2">
						{#each files[cli] as f (f.path)}
							<li class="rounded-lg border border-gray-200 dark:border-gray-700 px-3 py-2">
								<p class="text-meta">{i18n.t(f.key as never)}</p>
								<p class="font-mono text-xs text-gray-700 dark:text-gray-200 break-all flex items-center gap-1.5"><FileCode2 class="w-3.5 h-3.5 shrink-0 text-gray-400" aria-hidden="true" />{f.path}</p>
							</li>
						{/each}
					</ul>
					{#if sum.divs.length}
						<p class="text-body">
							{i18n.t('sync.clis.willDo')}
							{#if sum.add}<span class="chip border-emerald-300 text-emerald-700 dark:border-emerald-800 dark:text-emerald-300 ml-1">+{sum.add} {i18n.t('sync.clis.toAdd')}</span>{/if}
							{#if sum.update}<span class="chip border-primary-300 text-primary-700 dark:border-primary-700 dark:text-primary-300 ml-1">~{sum.update} {i18n.t('sync.clis.toUpdate')}</span>{/if}
							{#if sum.remove.length}<span class="chip border-red-300 text-red-700 dark:border-red-800 dark:text-red-300 ml-1">−{sum.remove.length} {i18n.t('sync.clis.toRemove')}</span>{/if}
							{#if sum.other}<span class="chip border-gray-300 text-gray-600 dark:border-gray-600 dark:text-gray-300 ml-1">{sum.other} {i18n.t('sync.clis.other')}</span>{/if}
						</p>
						<details class="group" open={sum.divs.length <= 8}>
							<summary class="text-body font-medium cursor-pointer flex items-center gap-1.5 select-none">
								<ChevronDown class="w-4 h-4 transition-transform group-open:rotate-180" aria-hidden="true" />{i18n.t('sync.clis.seeEach', { n: sum.divs.length })}
							</summary>
							<div class="mt-3 space-y-3">
								<DivergenceList divergences={sum.divs} hideCli />
							</div>
						</details>
					{/if}
				</section>
			{/each}
		{/if}
	</div>
</div>

<SidePanel open={!!panel} title={panel ? i18n.t('sync.clis.matchAllTitle', { cli: CLI_LABEL[panel.cli] }) : ''} onClose={() => (panel = null)}>
	{#if panel?.loading}
		<Skeleton rows={5} />
	{:else if panel && panel.changes.length === 0}
		<p class="text-body">{i18n.t('sync.panel.nothing')}</p>
	{:else if panel}
		{@const rm = summary(panel.cli).remove}
		{#if rm.length}
			<div class="rounded-lg border border-red-300 dark:border-red-800 bg-red-50 dark:bg-red-950/40 p-3 text-body space-y-1">
				<p class="font-medium text-red-800 dark:text-red-200">{i18n.t('sync.clis.willRemove', { n: rm.length, cli: CLI_LABEL[panel.cli] })}</p>
				<p class="text-red-800 dark:text-red-200 break-words">{[...new Set(rm.map((d) => d.name))].join(', ')}</p>
				<p class="text-meta">{i18n.t('sync.clis.keepHint')}</p>
			</div>
		{/if}
		{#each panel.changes as ch (ch.path)}
			<p class="text-body font-medium break-all">{ch.path}</p>
			<DiffView diff={ch.diff} />
		{/each}
	{/if}
	{#snippet footer()}
		<button class="btn btn-ghost" onclick={() => (panel = null)}>{i18n.t('sync.panel.cancel')}</button>
		<button class="btn btn-primary" disabled={!panel || panel.loading || panel.changes.length === 0} onclick={confirm}>{i18n.t('sync.panel.apply')}</button>
	{/snippet}
</SidePanel>
