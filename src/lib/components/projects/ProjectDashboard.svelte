<script lang="ts">
	import type { Project } from '$lib/types';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { projectsStore, notifications } from '$lib/stores';
	import { i18n } from '$lib/i18n';
	import { invoke } from '@tauri-apps/api/core';
	import { FolderOpen, ArrowLeft, ExternalLink } from 'lucide-svelte';
	import ProjectToolsPanel from './ProjectToolsPanel.svelte';
	import ProjectSettingsPanel from './ProjectSettingsPanel.svelte';
	import ProjectSyncPanel from '$lib/components/sync/ProjectSyncPanel.svelte';

	type Props = {
		project: Project;
	};

	let { project: projectProp }: Props = $props();

	let project = $derived(projectsStore.getProjectById(projectProp.id) ?? projectProp);

	type Tab = 'mcps' | 'skills' | 'agents' | 'commands' | 'hooks' | 'settings';
	const tabs: { id: Tab; key: string; tip: string }[] = [
		{ id: 'mcps', key: 'sync.project.tabMcps', tip: 'tip.ptab.mcps' },
		{ id: 'skills', key: 'nav.skills', tip: 'tip.ptab.skills' },
		{ id: 'agents', key: 'nav.agents', tip: 'tip.ptab.agents' },
		{ id: 'commands', key: 'nav.commands', tip: 'tip.ptab.commands' },
		{ id: 'hooks', key: 'nav.hooks', tip: 'tip.ptab.hooks' },
		{ id: 'settings', key: 'nav.settings', tip: 'tip.ptab.settings' }
	];

	let activeTab = $derived.by<Tab>(() => {
		const t = $page.url.searchParams.get('tab');
		if (t === 'tools') return 'skills'; // old links
		return tabs.some((x) => x.id === t) ? (t as Tab) : 'mcps';
	});
	let activeSection = $derived($page.url.searchParams.get('section') ?? 'models');

	function switchTab(tab: Tab) {
		goto(`/projects/${project.id}?tab=${tab}`, { replaceState: true, noScroll: true });
	}

	function handleSectionChange(section: string) {
		goto(`/projects/${project.id}?tab=settings&section=${section}`, { replaceState: true });
	}

	async function handleOpenFolder() {
		try {
			await invoke('open_folder', { path: project.path });
		} catch (err) {
			notifications.error(i18n.t('projComp.openFolderFailed'));
			console.error(err);
		}
	}

	function onTabKey(e: KeyboardEvent, i: number) {
		if (e.key !== 'ArrowRight' && e.key !== 'ArrowLeft') return;
		e.preventDefault();
		const next = tabs[(i + (e.key === 'ArrowRight' ? 1 : tabs.length - 1)) % tabs.length];
		switchTab(next.id);
		queueMicrotask(() => document.getElementById(`ptab-${next.id}`)?.focus());
	}
</script>

<div class="flex flex-col h-full">
	<div class="border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800">
		<div class="flex items-center gap-4 px-6 py-4">
			<button onclick={() => goto('/projects')} class="btn btn-ghost p-2" title={i18n.t('project.backToProjects')} aria-label={i18n.t('project.backToProjects')}>
				<ArrowLeft class="w-5 h-5" aria-hidden="true" />
			</button>
			<div class="w-10 h-10 rounded-xl bg-primary-50 dark:bg-primary-900/40 flex items-center justify-center flex-shrink-0">
				<FolderOpen class="w-5 h-5 text-primary-600 dark:text-primary-300" aria-hidden="true" />
			</div>
			<div class="flex-1 min-w-0">
				<h1 class="text-title truncate">{project.name}</h1>
				<p class="text-meta font-mono truncate" title={project.path}>{project.path}</p>
			</div>
			<button onclick={handleOpenFolder} class="btn btn-ghost text-sm" title={i18n.t('projComp.openFolderTitle')}>
				<ExternalLink class="w-4 h-4" aria-hidden="true" />
				<span class="hidden sm:inline ml-1.5">{i18n.t('global.openFolder')}</span>
			</button>
		</div>

		<div role="tablist" aria-label={i18n.t('sync.project.tabsLabel')} class="flex px-6 overflow-x-auto">
			{#each tabs as t, i (t.id)}
				<button id="ptab-{t.id}" role="tab" aria-selected={activeTab === t.id} tabindex={activeTab === t.id ? 0 : -1}
					data-tip={i18n.t(t.tip as never)} onclick={() => switchTab(t.id)} onkeydown={(e) => onTabKey(e, i)}
					class="px-4 py-2.5 text-sm font-medium border-b-2 whitespace-nowrap transition-colors {activeTab === t.id
						? 'border-primary-500 text-primary-700 dark:text-primary-300'
						: 'border-transparent text-gray-500 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-100'}">
					{i18n.t(t.key as never)}
				</button>
			{/each}
		</div>
	</div>

	<div class="flex-1 overflow-hidden flex flex-col" role="tabpanel" aria-labelledby="ptab-{activeTab}">
		{#if activeTab === 'mcps'}
			<ProjectSyncPanel path={project.path} />
		{:else if activeTab === 'settings'}
			<ProjectSettingsPanel {project} {activeSection} onSectionChange={handleSectionChange} />
		{:else}
			<ProjectToolsPanel {project} tab={activeTab} hideTabs />
		{/if}
	</div>
</div>
