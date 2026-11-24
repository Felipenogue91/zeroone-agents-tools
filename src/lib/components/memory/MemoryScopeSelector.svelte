<script lang="ts">
	import type { MemoryScope, AllMemoryFiles } from '$lib/types';
	import { i18n } from '$lib/i18n';
	import { User, FolderOpen, FileText } from 'lucide-svelte';

	type Props = {
		selectedScope: MemoryScope;
		memoryFiles: AllMemoryFiles | null;
		hasProject: boolean;
		onselect: (scope: MemoryScope) => void;
	};

	let { selectedScope, memoryFiles, hasProject, onselect }: Props = $props();

	const scopeLabels = $derived<Record<MemoryScope, { label: string; description: string }>>({
		user: {
			label: i18n.t('settingsWrapper.scopeUser'),
			description: i18n.t('memoryComp.scopeUserDesc')
		},
		project: {
			label: i18n.t('settingsWrapper.scopeProject'),
			description: i18n.t('memoryComp.scopeProjectDesc')
		},
		local: {
			label: i18n.t('settingsWrapper.scopeLocal'),
			description: i18n.t('memoryComp.scopeLocalDesc')
		}
	});

	const scopes: { key: MemoryScope; icon: typeof User }[] = [
		{ key: 'user', icon: User },
		{ key: 'project', icon: FolderOpen },
		{ key: 'local', icon: FileText }
	];

	function fileExists(scope: MemoryScope): boolean {
		if (!memoryFiles) return false;
		switch (scope) {
			case 'user':
				return memoryFiles.user.exists;
			case 'project':
				return memoryFiles.project?.exists ?? false;
			case 'local':
				return memoryFiles.local?.exists ?? false;
		}
	}
</script>

<div class="flex gap-1 bg-gray-100 dark:bg-gray-700/50 rounded-lg p-1">
	{#each scopes as { key, icon }}
		{@const isDisabled = key !== 'user' && !hasProject}
		{@const isActive = selectedScope === key}
		{@const exists = fileExists(key)}
		<button
			onclick={() => onselect(key)}
			disabled={isDisabled}
			class="flex items-center gap-2 px-4 py-2 rounded-md text-sm font-medium transition-colors flex-1
				{isActive
				? 'bg-white dark:bg-gray-600 text-gray-900 dark:text-white shadow-sm'
				: isDisabled
					? 'text-gray-400 dark:text-gray-500 cursor-not-allowed'
					: 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white'}"
			title={scopeLabels[key].description}
		>
			<svelte:component this={icon} class="w-4 h-4" />
			{scopeLabels[key].label}
			<span
				class="w-2 h-2 rounded-full {exists
					? 'bg-green-500'
					: 'bg-gray-300 dark:bg-gray-500'}"
				title={exists ? i18n.t('memoryComp.fileExists') : i18n.t('memoryComp.fileNotExists')}
			></span>
		</button>
	{/each}
</div>
