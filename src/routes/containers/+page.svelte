<script lang="ts">
	import { Header } from '$lib/components/layout';
	import { ContainerList, ContainerForm, ContainerDetail } from '$lib/components/containers';
	import { ConfirmDialog } from '$lib/components/shared';
	import { containerLibrary, notifications } from '$lib/stores';
	import { i18n } from '$lib/i18n';
	import type { Container, CreateContainerRequest } from '$lib/types';
	import { Plus, LayoutTemplate, Server, Search } from 'lucide-svelte';
	import { onMount, onDestroy } from 'svelte';

	let showAddContainer = $state(false);
	let editingContainer = $state<Container | null>(null);
	let deletingContainer = $state<Container | null>(null);
	let viewingContainer = $state<Container | null>(null);

	onMount(async () => {
		await containerLibrary.checkDocker();
		await containerLibrary.refreshAllStatuses();
		containerLibrary.startStatusPolling();
	});

	onDestroy(() => {
		containerLibrary.stopStatusPolling();
	});

	async function handleCreate(values: CreateContainerRequest) {
		try {
			await containerLibrary.create(values);
			showAddContainer = false;
			notifications.success(i18n.t('containers.created'));
		} catch (err) {
			notifications.error(i18n.t('containers.createFailed', { err: String(err) }));
		}
	}

	async function handleUpdate(values: CreateContainerRequest) {
		if (!editingContainer) return;
		try {
			await containerLibrary.update(editingContainer.id, values);
			editingContainer = null;
			notifications.success(i18n.t('containers.updated'));
		} catch (err) {
			notifications.error(i18n.t('containers.updateFailed', { err: String(err) }));
		}
	}

	async function handleDelete() {
		if (!deletingContainer) return;
		try {
			await containerLibrary.delete(deletingContainer.id);
			notifications.success(i18n.t('containers.deleted'));
		} catch (err) {
			notifications.error(i18n.t('containers.deleteFailed', { err: String(err) }));
		} finally {
			deletingContainer = null;
		}
	}
</script>

<Header
	title={i18n.t('page.containers.title')}
	subtitle={i18n.t('page.containers.subtitle')}
/>

<div class="flex-1 overflow-auto p-6">
	<div class="flex items-center justify-between mb-6">
		<div class="flex items-center gap-3">
			<div class="relative">
				<Search class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" aria-hidden="true" />
				<input
					type="text"
					bind:value={containerLibrary.searchQuery}
					placeholder={i18n.t('containers.searchPlaceholder')}
					class="input pl-9 w-64"
				/>
			</div>
			<select bind:value={containerLibrary.selectedType}
				class="input w-auto">
				<option value="all">{i18n.t('containers.allTypes', { count: containerLibrary.containerCount.total })}</option>
				<option value="docker">{i18n.t('containers.dockerCount', { count: containerLibrary.containerCount.docker })}</option>
				<option value="devcontainer">{i18n.t('containers.devContainerCount', { count: containerLibrary.containerCount.devcontainer })}</option>
				<option value="custom">{i18n.t('containers.customCount', { count: containerLibrary.containerCount.custom })}</option>
			</select>
			{#if containerLibrary.dockerAvailable === false}
				<span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-red-100 text-red-700 dark:bg-red-900/50 dark:text-red-400">{i18n.t('containers.dockerUnavailable')}</span>
			{:else if containerLibrary.dockerAvailable === true}
				<span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-700 dark:bg-green-900/50 dark:text-green-400">{i18n.t('containers.dockerConnected')}</span>
			{/if}
		</div>
		<div class="flex gap-2">
			<a href="/containers/templates"
				class="btn btn-secondary gap-2">
				<LayoutTemplate class="w-4 h-4" aria-hidden="true" />
				{i18n.t('containers.templates')}
			</a>
			<a href="/containers/hosts"
				class="btn btn-secondary gap-2">
				<Server class="w-4 h-4" aria-hidden="true" />
				{i18n.t('containers.hosts')}
			</a>
			<button onclick={() => showAddContainer = true} class="btn btn-primary gap-2">
				<Plus class="w-4 h-4" aria-hidden="true" />
				{i18n.t('containers.newContainer')}
			</button>
		</div>
	</div>

	<ContainerList
		onEdit={(container) => editingContainer = container}
		onDelete={(container) => deletingContainer = container}
		onView={(container) => viewingContainer = container}
	/>
</div>

<!-- Add Container Modal -->
{#if showAddContainer}
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50" role="dialog" aria-modal="true"
		onkeydown={(e) => e.key === 'Escape' && (showAddContainer = false)}
		onclick={(e) => e.target === e.currentTarget && (showAddContainer = false)}>
		<div class="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-2xl w-full mx-4 max-h-[90vh] overflow-auto">
			<div class="p-6">
				<h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-6">{i18n.t('containers.newContainer')}</h2>
				<ContainerForm onSubmit={handleCreate} onCancel={() => showAddContainer = false} />
			</div>
		</div>
	</div>
{/if}

<!-- Edit Container Modal -->
{#if editingContainer}
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50" role="dialog" aria-modal="true"
		onkeydown={(e) => e.key === 'Escape' && (editingContainer = null)}
		onclick={(e) => e.target === e.currentTarget && (editingContainer = null)}>
		<div class="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-2xl w-full mx-4 max-h-[90vh] overflow-auto">
			<div class="p-6">
				<h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-6">{i18n.t('containers.editContainer')}</h2>
				<ContainerForm container={editingContainer} onSubmit={handleUpdate} onCancel={() => editingContainer = null} />
			</div>
		</div>
	</div>
{/if}

<!-- Detail Modal -->
{#if viewingContainer}
	<ContainerDetail container={viewingContainer} onClose={() => viewingContainer = null} />
{/if}

<!-- Delete Confirmation -->
<ConfirmDialog
	open={!!deletingContainer}
	title={i18n.t('containers.deleteContainer')}
	message={i18n.t('containers.deleteConfirm', { name: deletingContainer?.name ?? '' })}
	confirmText={i18n.t('common.delete')}
	onConfirm={handleDelete}
	onCancel={() => deletingContainer = null}
/>
