<script lang="ts">
	import { untrack } from 'svelte';
	import { Plus, GitBranch, CircleCheck, TriangleAlert, Power, Info } from 'lucide-svelte';
	import { syncStore } from '$lib/stores/syncStore.svelte';
	import { i18n } from '$lib/i18n';
	import type { Cli, ProjectServer, ProjectSyncView } from '$lib/types/sync';
	import CliChips from './CliChips.svelte';
	import Skeleton from './Skeleton.svelte';
	import AddMcpPanel from './AddMcpPanel.svelte';

	type Props = { path: string };
	let { path }: Props = $props();

	let view = $state<ProjectSyncView | null>(null);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let adding = $state(false);

	async function load() {
		try {
			view = await syncStore.loadProject(path);
			error = null;
		} catch (e) {
			error = String(e);
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		path;
		loading = true;
		load();
	});

	// refresh when the watcher reports changes (resources reload)
	$effect(() => {
		syncStore.resources;
		untrack(() => {
			if (view) load();
		});
	});

	let own = $derived(view?.servers.filter((s) => !s.inherited) ?? []);
	let inherited = $derived(view?.servers.filter((s) => s.inherited) ?? []);

	function allOff(s: ProjectServer) {
		return ['claude', 'codex', 'agy'].every((c) => s.targets[c as Cli]?.state === 'off');
	}

	async function toggleCli(s: ProjectServer, cli: Cli, enabled: boolean) {
		await syncStore.setTarget(s.name, cli, enabled, path);
		await load();
	}

	async function toggleAll(s: ProjectServer) {
		await syncStore.setEnabled(path, s.name, allOff(s));
		await load();
	}

	// what each CLI can do with a global server in one project
	let inheritedLocks = $derived<Partial<Record<Cli, string>>>({
		agy: i18n.t('sync.project.agyCantInherit'),
		...(view?.managed ? {} : { codex: i18n.t('sync.project.codexNeedsSync') })
	});

	function inheritedAllOff(s: ProjectServer) {
		return (['claude', 'codex'] as Cli[]).every((c) => s.targets[c]?.state !== 'on');
	}

	async function toggleInheritedAll(s: ProjectServer) {
		const turnOn = inheritedAllOff(s);
		for (const cli of ['claude', 'codex'] as Cli[]) {
			if (inheritedLocks[cli]) continue;
			const st = s.targets[cli]?.state;
			if (turnOn ? st === 'off' : st === 'on') await syncStore.setTarget(s.name, cli, turnOn, path);
		}
		await load();
	}

	async function manage(managed: boolean) {
		await syncStore.manageProject(path, managed);
		await load();
	}
</script>

{#if loading && !view}
	<div class="p-6"><Skeleton rows={6} /></div>
{:else if error}
	<div class="p-6 text-body">
		<p class="font-medium">{i18n.t('sync.project.loadFailed')}</p>
		<p class="text-meta break-all mt-1">{error}</p>
	</div>
{:else if view}
	<div class="flex-1 overflow-y-auto">
		<div class="max-w-5xl mx-auto px-6 py-4 space-y-4">
			{#if !view.managed}
				<section class="card flex items-start gap-3">
					<Info class="w-5 h-5 text-primary-500 shrink-0 mt-0.5" aria-hidden="true" />
					<div class="flex-1">
						<p class="text-body font-medium">{i18n.t('sync.project.unmanagedTitle')}</p>
						<p class="text-meta mt-1">{i18n.t('sync.project.unmanagedBody')}</p>
					</div>
					<button class="btn btn-primary" disabled={syncStore.isBusy(`manage:${path}`)} onclick={() => manage(true)}>
						{i18n.t('sync.project.manage')}
					</button>
				</section>
			{:else}
				<section class="flex items-center gap-3 flex-wrap">
					{#if view.pending.length}
						<span class="chip border-amber-300 text-amber-700 dark:border-amber-700 dark:text-amber-300">
							<TriangleAlert class="w-3 h-3" aria-hidden="true" />{i18n.t('sync.project.pending', { clis: view.pending.join(', ') })}
						</span>
						<button class="btn btn-secondary text-xs px-3 py-1" onclick={async () => { await syncStore.applyAll(path); await load(); }}>
							{i18n.t('sync.project.applyNow')}
						</button>
					{:else}
						<span class="chip border-emerald-300 text-emerald-700 dark:border-emerald-800 dark:text-emerald-300">
							<CircleCheck class="w-3 h-3" aria-hidden="true" />{i18n.t('sync.project.inSync')}
						</span>
					{/if}
					<span class="text-meta">{i18n.t('sync.project.autosave')}</span>
				</section>
			{/if}

			<!-- Own servers -->
			<section class="card p-0 overflow-hidden">
				<header class="flex items-center px-4 py-3 border-b border-gray-200 dark:border-gray-700">
					<div class="flex-1">
						<h3 class="text-body font-semibold">{i18n.t('sync.project.own')} <span class="text-meta font-normal">({own.length})</span></h3>
						<p class="text-meta">{i18n.t('sync.project.ownHint')}</p>
					</div>
					<button class="btn btn-secondary text-xs px-3 py-1.5" onclick={() => (adding = true)}>
						<Plus class="w-3.5 h-3.5 mr-1" aria-hidden="true" />{i18n.t('sync.lib.addMcp')}
					</button>
				</header>
				{#if own.length === 0}
					<p class="px-4 py-6 text-meta text-center">{i18n.t('sync.project.noOwn')}</p>
				{:else}
					<ul>
						{#each own as s (s.name)}
							<li class="row-dense {allOff(s) ? 'opacity-60' : ''}">
								<div class="flex-1 min-w-0">
									<p class="text-body truncate"><span class="font-medium text-gray-900 dark:text-white">{s.name}</span>
										{#if s.overridesGlobal}<span class="text-meta"> · {i18n.t('sync.project.overridesGlobal')}</span>{/if}</p>
									{#if s.secretRefs.length}<p class="text-meta truncate">{s.secretRefs.map((v) => '${' + v + '}').join(' ')}</p>{/if}
								</div>
								<CliChips targets={s.targets} disabled={syncStore.isBusy(`${path}:${s.name}`)}
									onToggle={(cli, enabled) => toggleCli(s, cli, enabled)} />
								<button class="btn btn-ghost p-1.5" disabled={syncStore.isBusy(`${path}:${s.name}`)} onclick={() => toggleAll(s)}
									title={allOff(s) ? i18n.t('sync.project.turnOnAll') : i18n.t('sync.project.turnOffAll')}
									aria-label={allOff(s) ? i18n.t('sync.project.turnOnAll') : i18n.t('sync.project.turnOffAll')}>
									<Power class="w-4 h-4 {allOff(s) ? 'text-gray-400' : 'text-primary-600 dark:text-primary-300'}" aria-hidden="true" />
								</button>
							</li>
						{/each}
					</ul>
				{/if}
			</section>

			<!-- Inherited from global -->
			{#if inherited.length}
				<section class="card p-0 overflow-hidden">
					<header class="px-4 py-3 border-b border-gray-200 dark:border-gray-700">
						<h3 class="text-body font-semibold">{i18n.t('sync.project.inherited')} <span class="text-meta font-normal">({inherited.length})</span></h3>
						<p class="text-meta">{i18n.t('sync.project.inheritedHint')}</p>
					</header>
					<ul>
						{#each inherited as s (s.name)}
							<li class="row-dense {inheritedAllOff(s) ? 'opacity-60' : ''}">
								<p class="text-body flex-1 truncate"><span class="font-medium text-gray-900 dark:text-white">{s.name}</span> <span class="text-meta">· {i18n.t('sync.project.inheritedTag')}</span></p>
								<CliChips targets={s.targets} locked={inheritedLocks} disabled={syncStore.isBusy(`${path}:${s.name}`)}
									onToggle={(cli, enabled) => toggleCli(s, cli, enabled)} />
								<button class="btn btn-ghost p-1.5" disabled={syncStore.isBusy(`${path}:${s.name}`)} onclick={() => toggleInheritedAll(s)}
									title={inheritedAllOff(s) ? i18n.t('sync.project.inheritedOn') : i18n.t('sync.project.inheritedOff')}
									aria-label={inheritedAllOff(s) ? i18n.t('sync.project.inheritedOn') : i18n.t('sync.project.inheritedOff')}>
									<Power class="w-4 h-4 {inheritedAllOff(s) ? 'text-gray-400' : 'text-primary-600 dark:text-primary-300'}" aria-hidden="true" />
								</button>
							</li>
						{/each}
					</ul>
				</section>
			{/if}

			<!-- Worktrees -->
			{#if view.managed && view.worktrees.length > 1}
				<section class="card space-y-2">
					<h3 class="text-body font-semibold flex items-center gap-2"><GitBranch class="w-4 h-4" aria-hidden="true" />{i18n.t('sync.project.worktrees')}</h3>
					<ul class="space-y-1">
						{#each view.worktrees as w (w.path)}
							<li class="flex items-center gap-2 text-body">
								{#if w.agyInSync}
									<CircleCheck class="w-4 h-4 text-emerald-500" aria-label={i18n.t('sync.project.inSync')} />
								{:else}
									<TriangleAlert class="w-4 h-4 text-amber-500" aria-label={i18n.t('sync.project.agyBehind')} />
								{/if}
								<span class="font-medium">{w.label}</span>
								<span class="text-meta truncate">{w.path}</span>
							</li>
						{/each}
					</ul>
				</section>
			{/if}

			{#if view.managed}
				<p class="text-meta">
					<button class="underline hover:text-gray-700 dark:hover:text-gray-200" onclick={() => manage(false)}>{i18n.t('sync.project.unmanage')}</button>
				</p>
			{/if}
		</div>
	</div>
{/if}

<AddMcpPanel open={adding} projects={[path]} defaultProject={path} onClose={() => { adding = false; load(); }} />
