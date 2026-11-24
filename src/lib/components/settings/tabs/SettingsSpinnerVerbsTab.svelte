<script lang="ts">
	import { onMount } from 'svelte';
	import { SpinnerVerbList, SpinnerVerbForm } from '$lib/components/spinnerverbs';
	import { ConfirmDialog } from '$lib/components/shared';
	import { spinnerVerbLibrary, notifications } from '$lib/stores';
	import { i18n } from '$lib/i18n';
	import type { SpinnerVerb } from '$lib/types';
	import { Plus, RefreshCw } from 'lucide-svelte';

	let showAddVerb = $state(false);
	let editingVerb = $state<SpinnerVerb | null>(null);
	let deletingVerb = $state<SpinnerVerb | null>(null);
	let isSyncing = $state(false);

	onMount(async () => {
		await spinnerVerbLibrary.load();
	});

	async function handleCreate(verb: string) {
		try {
			await spinnerVerbLibrary.create(verb);
			showAddVerb = false;
			notifications.success(i18n.t('settingsComp.spinnerVerbAdded'));
		} catch (err) {
			notifications.error(i18n.t('settingsComp.spinnerVerbAddFailed', { err: String(err) }));
		}
	}

	async function handleUpdate(verb: string) {
		if (!editingVerb) return;
		try {
			await spinnerVerbLibrary.update(editingVerb.id, verb, editingVerb.isEnabled);
			editingVerb = null;
			notifications.success(i18n.t('settingsComp.spinnerVerbUpdated'));
		} catch (err) {
			notifications.error(i18n.t('settingsComp.spinnerVerbUpdateFailed', { err: String(err) }));
		}
	}

	async function handleDelete() {
		if (!deletingVerb) return;
		try {
			await spinnerVerbLibrary.delete(deletingVerb.id);
			notifications.success(i18n.t('settingsComp.spinnerVerbDeleted'));
		} catch (err) {
			notifications.error(i18n.t('settingsComp.spinnerVerbDeleteFailed'));
		} finally {
			deletingVerb = null;
		}
	}

	async function handleModeChange(e: Event) {
		const target = e.target as HTMLSelectElement;
		const mode = target.value as 'append' | 'replace';
		try {
			await spinnerVerbLibrary.setMode(mode);
			notifications.success(i18n.t('settingsComp.spinnerModeChanged', { mode }));
		} catch (err) {
			notifications.error(i18n.t('settingsComp.spinnerModeChangeFailed'));
		}
	}

	async function handleSync() {
		isSyncing = true;
		try {
			await spinnerVerbLibrary.sync();
			notifications.success(i18n.t('settingsComp.spinnerSynced'));
		} catch (err) {
			notifications.error(i18n.t('settingsComp.spinnerSyncFailed', { err: String(err) }));
		} finally {
			isSyncing = false;
		}
	}
</script>

<!-- Mode & Actions Bar -->
<div class="flex items-center justify-between mb-6">
	<div class="flex items-center gap-4">
		<label
			for="spinner-mode"
			class="text-sm font-medium text-gray-700 dark:text-gray-300"
		>
			{i18n.t('settingsComp.spinnerModeLabel')}
		</label>
		<select
			id="spinner-mode"
			value={spinnerVerbLibrary.mode}
			onchange={handleModeChange}
			class="px-3 py-1.5 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm focus:ring-2 focus:ring-primary-500 focus:border-transparent"
		>
			<option value="append">{i18n.t('settingsComp.spinnerModeAppend')}</option>
			<option value="replace">{i18n.t('settingsComp.spinnerModeReplace')}</option>
		</select>
	</div>

	<div class="flex items-center gap-3">
		<button
			onclick={handleSync}
			disabled={isSyncing}
			class="btn btn-secondary flex items-center gap-2"
		>
			<RefreshCw class="w-4 h-4 {isSyncing ? 'animate-spin' : ''}" />
			{i18n.t('settingsComp.syncToSettings')}
		</button>
		<button onclick={() => (showAddVerb = true)} class="btn btn-primary">
			<Plus class="w-4 h-4 mr-2" />
			{i18n.t('settingsComp.addVerb')}
		</button>
	</div>
</div>

<!-- Verb List -->
<SpinnerVerbList
	onEdit={(verb) => (editingVerb = verb)}
	onDelete={(verb) => (deletingVerb = verb)}
/>

<!-- Add Verb Modal -->
{#if showAddVerb}
	<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
		<div class="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full mx-4">
			<div class="p-6">
				<h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-6">
					{i18n.t('settingsComp.addSpinnerVerbTitle')}
				</h2>
				<SpinnerVerbForm onSubmit={handleCreate} onCancel={() => (showAddVerb = false)} />
			</div>
		</div>
	</div>
{/if}

<!-- Edit Verb Modal -->
{#if editingVerb}
	<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
		<div class="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full mx-4">
			<div class="p-6">
				<h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-6">
					{i18n.t('settingsComp.editSpinnerVerbTitle')}
				</h2>
				<SpinnerVerbForm
					initialValues={editingVerb}
					onSubmit={handleUpdate}
					onCancel={() => (editingVerb = null)}
				/>
			</div>
		</div>
	</div>
{/if}

<ConfirmDialog
	open={!!deletingVerb}
	title={i18n.t('settingsComp.deleteSpinnerVerbTitle')}
	message={i18n.t('settingsComp.deleteSpinnerVerbConfirm', { verb: deletingVerb?.verb ?? '' })}
	confirmText={i18n.t('common.delete')}
	onConfirm={handleDelete}
	onCancel={() => (deletingVerb = null)}
/>
