<script lang="ts">
	import { onMount } from 'svelte';
	import { TriangleAlert, CircleCheck, FileCode2, Eye } from 'lucide-svelte';
	import { Header } from '$lib/components/layout';
	import { syncStore } from '$lib/stores/syncStore.svelte';
	import { notifications } from '$lib/stores';
	import { i18n } from '$lib/i18n';
	import CliChip from '$lib/components/sync/CliChip.svelte';
	import DiffView from '$lib/components/sync/DiffView.svelte';
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
				{@const pending = syncStore.health.divergences.filter((d) => d.cli === cli).length}
				<section class="card space-y-3" aria-labelledby="cli-{cli}">
					<div class="flex items-center gap-3 flex-wrap">
						<CliChip {cli} label />
						<h2 id="cli-{cli}" class="text-title flex-1">{CLI_LABEL[cli]}</h2>
						{#if pending}
							<span class="chip border-amber-300 text-amber-700 dark:border-amber-700 dark:text-amber-300"><TriangleAlert class="w-3 h-3" aria-hidden="true" />{i18n.t('sync.home.cliPending', { n: pending })}</span>
						{:else}
							<span class="chip border-emerald-300 text-emerald-700 dark:border-emerald-800 dark:text-emerald-300"><CircleCheck class="w-3 h-3" aria-hidden="true" />{i18n.t('sync.home.cliOk')}</span>
						{/if}
						<button class="btn btn-secondary" onclick={() => openReapply(cli)}>
							<Eye class="w-4 h-4 mr-1.5" aria-hidden="true" />{i18n.t('sync.clis.reapply')}
						</button>
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
				</section>
			{/each}
		{/if}
	</div>
</div>

<SidePanel open={!!panel} title={panel ? i18n.t('sync.clis.reapplyTitle', { cli: CLI_LABEL[panel.cli] }) : ''} onClose={() => (panel = null)}>
	{#if panel?.loading}
		<Skeleton rows={5} />
	{:else if panel && panel.changes.length === 0}
		<p class="text-body">{i18n.t('sync.panel.nothing')}</p>
	{:else if panel}
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
