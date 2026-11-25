<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { CircleCheck, TriangleAlert, Eye, KeyRound, ShieldAlert, FolderTree } from 'lucide-svelte';
	import { Header } from '$lib/components/layout';
	import { syncStore } from '$lib/stores/syncStore.svelte';
	import { notifications } from '$lib/stores';
	import { i18n } from '$lib/i18n';
	import type { ChangeView, PlanView } from '$lib/types/sync';
	import { CLIS, CLI_LABEL } from '$lib/types/sync';
	import CliChip from './CliChip.svelte';
	import DivergenceList from './DivergenceList.svelte';
	import DiffView from './DiffView.svelte';
	import SidePanel from './SidePanel.svelte';
	import Skeleton from './Skeleton.svelte';

	let panel = $state<{
		title: string;
		subtitle?: string;
		changes: ChangeView[];
		loading: boolean;
		apply: () => Promise<unknown>;
	} | null>(null);

	onMount(() => {
		syncStore.loadHealth();
		syncStore.loadManagement();
	});

	let health = $derived(syncStore.health);

	let perCli = $derived(
		CLIS.map((cli) => ({
			cli,
			count: (health?.divergences ?? []).filter((d) => d.cli === cli).length,
			version: health?.cliVersions.find((v) => v.cli === cli)
		}))
	);

	let checkedAt = $derived(
		health ? new Date(health.checkedAt).toLocaleTimeString(i18n.locale, { hour: '2-digit', minute: '2-digit' }) : ''
	);

	async function openApplyAll() {
		panel = {
			title: i18n.t('sync.home.reviewAll'),
			subtitle: i18n.t('sync.home.reviewAllHint'),
			changes: [],
			loading: true,
			apply: () => syncStore.applyAll(null)
		};
		try {
			const plan: PlanView = await syncStore.previewAll(null);
			if (panel) panel = { ...panel, changes: plan.changes, loading: false };
		} catch (e) {
			notifications.add('error', i18n.t('sync.previewFailed'), { detail: String(e), duration: 8000 });
			panel = null;
		}
	}

	async function confirmPanel() {
		if (!panel) return;
		const run = panel.apply;
		panel = null;
		await run();
	}

	async function fixPermissions() {
		try {
			await invoke('fix_secrets_permissions');
			notifications.success(i18n.t('sync.secrets.permissionsFixed'));
			await syncStore.loadHealth();
		} catch (e) {
			notifications.add('error', i18n.t('sync.saveFailed'), { detail: String(e) });
		}
	}
</script>

<Header title={i18n.t('sync.home.title')} subtitle={i18n.t('sync.home.subtitle')} />

<div class="flex-1 overflow-y-auto">
	<div class="max-w-5xl mx-auto px-6 py-6 space-y-6">
		{#if !health && syncStore.isLoadingHealth}
			<div class="card"><Skeleton rows={4} /></div>
		{:else if !health && syncStore.error}
			<div class="card text-body">
				<p class="font-medium">{i18n.t('sync.home.loadFailed')}</p>
				<p class="text-meta mt-1 break-all">{syncStore.error}</p>
				<button data-tip={i18n.t('tip.home.tryAgain')} class="btn btn-secondary mt-3" onclick={() => syncStore.loadHealth()}>{i18n.t('sync.home.tryAgain')}</button>
			</div>
		{:else if health}
			{#if syncStore.management && !syncStore.management.externallyManaged && health.resources === 0}
				<section class="card">
					<div class="flex items-start gap-4">
						<FolderTree class="w-6 h-6 text-primary-500 shrink-0 mt-0.5" aria-hidden="true" />
						<div class="space-y-2">
							<h2 class="text-title">{i18n.t('sync.home.noAgentsTitle')}</h2>
							<p class="text-body">{i18n.t('sync.home.noAgentsBody')}</p>
							<pre class="font-mono text-xs text-gray-600 dark:text-gray-300 bg-gray-50 dark:bg-gray-900 rounded-lg p-3">~/.agents/mcp/global.json
~/.agents/mcp/sync.json
~/.agents/skills/</pre>
							<a data-tip={i18n.t('tip.home.goLibrary')} href="/library" class="btn btn-primary">{i18n.t('sync.home.goLibrary')}</a>
						</div>
					</div>
				</section>
			{/if}

			<!-- Status -->
			<section class="card flex items-center gap-4" aria-live="polite">
				{#if health.inSync}
					<CircleCheck class="w-10 h-10 text-emerald-500 shrink-0" aria-hidden="true" />
					<div class="flex-1">
						<h2 class="text-title">{i18n.t('sync.home.allSynced')}</h2>
						<p class="text-meta mt-1">{i18n.t('sync.home.summary', { n: health.resources, time: checkedAt })}</p>
					</div>
				{:else}
					<TriangleAlert class="w-10 h-10 text-amber-500 shrink-0" aria-hidden="true" />
					<div class="flex-1">
						<h2 class="text-title">
							{health.divergences.length > 0
								? i18n.t('sync.home.divergences', { n: health.divergences.length })
								: i18n.t('sync.home.attention')}
						</h2>
						<p class="text-meta mt-1">{i18n.t('sync.home.summary', { n: health.resources, time: checkedAt })}</p>
					</div>
					<button data-tip={i18n.t('tip.home.reviewAll')} class="btn btn-primary" onclick={openApplyAll}>
						<Eye class="w-4 h-4 mr-1.5" aria-hidden="true" />
						{i18n.t('sync.home.reviewAll')}
					</button>
				{/if}
			</section>

			<!-- Per CLI -->
			<section class="grid grid-cols-1 sm:grid-cols-3 gap-3" aria-label={i18n.t('sync.home.perCli')}>
				{#each perCli as row (row.cli)}
					<a data-tip={i18n.t('tip.home.cliCard')} href="/clis#cli-{row.cli}" class="card flex items-center gap-3 hover:border-primary-300 dark:hover:border-primary-700 transition-colors">
						<CliChip cli={row.cli} label />
						<div class="flex-1 min-w-0">
							<p class="text-body font-medium">{CLI_LABEL[row.cli]}</p>
							<p class="text-meta truncate">
								{row.version?.installed ? `v${row.version.installed}` : i18n.t('sync.cli.notInstalled')}
								· {row.count ? i18n.t('sync.home.cliPending', { n: row.count }) : i18n.t('sync.home.cliOk')}
							</p>
						</div>
						{#if row.version?.message}
							<span title={row.version.message}><TriangleAlert class="w-4 h-4 text-amber-500" aria-label={row.version.message} /></span>
						{/if}
					</a>
				{/each}
			</section>

			<!-- Secrets -->
			{#if health.missingSecrets.length || !health.secretsPermissionsOk}
				<section class="card space-y-2">
					<h3 class="text-body font-semibold flex items-center gap-2">
						<KeyRound class="w-4 h-4 text-amber-500" aria-hidden="true" />
						{i18n.t('sync.secrets.title')}
					</h3>
					{#if health.missingSecrets.length}
						<p class="text-body">{i18n.t('sync.secrets.missing')}</p>
						<div class="flex flex-wrap gap-1.5">
							{#each health.missingSecrets as v (v)}
								<code class="chip border-amber-300 text-amber-800 dark:border-amber-700 dark:text-amber-200 font-mono">{v}</code>
							{/each}
						</div>
						<p class="text-meta">{i18n.t('sync.secrets.howTo')}</p>
					{/if}
					{#if !health.secretsPermissionsOk}
						<div class="flex items-center gap-3">
							<ShieldAlert class="w-4 h-4 text-red-500" aria-hidden="true" />
							<p class="text-body flex-1">{i18n.t('sync.secrets.openPermissions')}</p>
							<button data-tip={i18n.t('tip.home.fixPermissions')} class="btn btn-secondary" onclick={fixPermissions}>{i18n.t('sync.secrets.fixPermissions')}</button>
						</div>
					{/if}
				</section>
			{/if}

			<!-- Divergences -->
			<DivergenceList divergences={health.divergences} />

			<!-- Engine report (same wording as mcp-sync --check) -->
			{#if health.report.length}
				<details class="card">
					<summary data-tip={i18n.t('tip.home.report')} class="text-body font-medium cursor-pointer">{i18n.t('sync.home.report')}</summary>
					<ul class="mt-3 space-y-2">
						{#each health.report as r (r.check)}
							<li class="text-body">
								<span aria-hidden="true">{r.status === 'ok' ? '✅' : '🟡'}</span> {r.summary}
								{#if r.detail}<p class="text-meta ml-6 break-words">{r.detail}</p>{/if}
							</li>
						{/each}
					</ul>
				</details>
			{/if}
		{/if}
	</div>
</div>

<SidePanel open={!!panel} title={panel?.title ?? ''} subtitle={panel?.subtitle} onClose={() => (panel = null)}>
	{#if panel?.loading}
		<Skeleton rows={6} />
	{:else if panel && panel.changes.length === 0}
		<p class="text-body">{i18n.t('sync.panel.nothing')}</p>
	{:else if panel}
		{#each panel.changes as ch (ch.path + ch.label)}
			<div class="space-y-2">
				<div class="flex items-center gap-2">
					<CliChip cli={ch.cli} label />
					<p class="text-body font-medium flex-1 truncate" title={ch.path}>{ch.label}</p>
					<span class="text-meta">{i18n.t(`sync.change.${ch.action}` as never)}</span>
				</div>
				<p class="text-meta break-all">{ch.path}</p>
				<DiffView diff={ch.diff} />
			</div>
		{/each}
	{/if}
	{#snippet footer()}
		<button class="btn btn-ghost" onclick={() => (panel = null)}>{i18n.t('sync.panel.cancel')}</button>
		<button data-tip={i18n.t('tip.sync.apply')} class="btn btn-primary" disabled={!panel || panel.loading || panel.changes.length === 0} onclick={confirmPanel}>
			{i18n.t('sync.panel.apply')}
		</button>
	{/snippet}
</SidePanel>
