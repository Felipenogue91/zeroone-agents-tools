<script lang="ts">
	import { syncStore } from '$lib/stores/syncStore.svelte';
	import { notifications } from '$lib/stores';
	import { i18n } from '$lib/i18n';
	import type { ChangeView, Divergence, DivergenceKind, HealthAction } from '$lib/types/sync';
	import { CLI_LABEL, scopeLabel } from '$lib/types/sync';
	import CliChip from './CliChip.svelte';
	import DiffView from './DiffView.svelte';
	import SidePanel from './SidePanel.svelte';
	import Skeleton from './Skeleton.svelte';

	type Props = {
		divergences: Divergence[];
		/** Hide the CLI chip on each row (list already filtered by CLI). */
		hideCli?: boolean;
	};
	let { divergences, hideCli = false }: Props = $props();

	const kindOrder: DivergenceKind[] = ['onlyInCli', 'missingInCli', 'drift', 'claudeLocalScope', 'agySkillsFolder'];

	let groups = $derived(
		kindOrder
			.map((kind) => ({ kind, items: divergences.filter((d) => d.kind === kind) }))
			.filter((g) => g.items.length > 0)
	);

	/** Label that says what the button really does for this kind. */
	function actionLabel(d: Divergence, action: HealthAction) {
		const cli = CLI_LABEL[d.cli];
		if (action === 'reapply') {
			if (d.kind === 'onlyInCli') return i18n.t('sync.item.removeFromCli', { cli });
			if (d.kind === 'missingInCli') return i18n.t('sync.item.addToCli', { cli });
			return i18n.t('sync.item.matchSource');
		}
		return i18n.t(`sync.action.${action}` as never);
	}

	/** What happens when clicking, in plain words (shown on a longer hover). */
	function actionTip(d: Divergence, action: HealthAction) {
		const cli = CLI_LABEL[d.cli];
		if (action === 'reapply') {
			if (d.kind === 'onlyInCli') return i18n.t('sync.tip.removeFromCli', { cli });
			if (d.kind === 'missingInCli') return i18n.t('sync.tip.addToCli', { cli });
			return i18n.t('sync.tip.matchSource', { cli });
		}
		return i18n.t(`sync.tip.${action}` as never);
	}

	let panel = $state<{ title: string; subtitle?: string; changes: ChangeView[]; loading: boolean; run: () => Promise<unknown> } | null>(null);

	async function open(d: Divergence, action: HealthAction) {
		panel = { title: actionLabel(d, action), subtitle: d.message, changes: [], loading: true, run: () => syncStore.applyAction(d, action) };
		try {
			const changes = await syncStore.previewAction(d, action);
			if (panel) panel = { ...panel, changes, loading: false };
		} catch (e) {
			notifications.add('error', i18n.t('sync.previewFailed'), { detail: String(e), duration: 8000 });
			panel = null;
		}
	}

	async function confirm() {
		if (!panel) return;
		const run = panel.run;
		panel = null;
		await run();
	}
</script>

{#each groups as group (group.kind)}
	<section class="card p-0 overflow-hidden">
		<header class="px-4 py-3 border-b border-gray-200 dark:border-gray-700">
			<h3 class="text-body font-semibold">{i18n.t(`sync.kind.${group.kind}` as never)} <span class="text-meta font-normal">({group.items.length})</span></h3>
			<p class="text-meta mt-0.5">{i18n.t(`sync.kindHint.${group.kind}` as never)}</p>
		</header>
		<ul>
			{#each group.items as d, i (d.cli + d.name + d.filePath + i)}
				<li class="row-dense">
					{#if !hideCli}<CliChip cli={d.cli} label />{/if}
					<div class="flex-1 min-w-0">
						<p class="text-body truncate"><span class="font-medium">{d.name}</span>
							<span class="text-meta"> · {scopeLabel(d.scope)}</span></p>
						<p class="text-meta truncate" title={d.filePath}>{d.filePath}</p>
					</div>
					<div class="flex items-center gap-1 shrink-0">
						{#each d.actions as action (action)}
							<button class="btn {action === 'import' || action === 'refreshLocalCopy' ? 'btn-secondary' : 'btn-ghost'} text-xs px-2 py-1" data-tip={actionTip(d, action)} onclick={() => open(d, action)}>
								{actionLabel(d, action)}
							</button>
						{/each}
					</div>
				</li>
			{/each}
		</ul>
	</section>
{/each}

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
		<button class="btn btn-primary" disabled={!panel || panel.loading || panel.changes.length === 0} onclick={confirm}>{i18n.t('sync.panel.apply')}</button>
	{/snippet}
</SidePanel>
