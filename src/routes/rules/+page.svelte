<script lang="ts">
	import { Header } from '$lib/components/layout';
	import { RuleLibrary, RuleForm } from '$lib/components/rules';
	import { ConfirmDialog } from '$lib/components/shared';
	import { ruleLibrary, notifications } from '$lib/stores';
	import { i18n } from '$lib/i18n';
	import type { Rule } from '$lib/types';
	import { Plus } from 'lucide-svelte';

	let showAddRule = $state(false);
	let editingRule = $state<Rule | null>(null);
	let deletingRule = $state<Rule | null>(null);

	async function handleCreateRule(values: any) {
		try {
			await ruleLibrary.create(values);
			showAddRule = false;
			notifications.success(i18n.t('rule.created'));
		} catch (err) {
			notifications.error(i18n.t('rule.createFailed'));
		}
	}

	async function handleUpdateRule(values: any) {
		if (!editingRule) return;
		try {
			await ruleLibrary.update(editingRule.id, values);
			editingRule = null;
			notifications.success(i18n.t('rule.updated'));
		} catch (err) {
			notifications.error(i18n.t('rule.updateFailed'));
		}
	}

	async function handleDeleteRule() {
		if (!deletingRule) return;
		try {
			await ruleLibrary.delete(deletingRule.id);
			notifications.success(i18n.t('rule.deleted'));
		} catch (err) {
			notifications.error(i18n.t('rule.deleteFailed'));
		} finally {
			deletingRule = null;
		}
	}
</script>

<Header
	title={i18n.t('page.rules.title')}
	subtitle={i18n.t('page.rules.subtitle')}
/>

<div class="flex-1 overflow-auto p-6">
	<div class="flex justify-end mb-6">
		<button onclick={() => (showAddRule = true)} class="btn btn-primary">
			<Plus class="w-4 h-4 mr-2" />
			{i18n.t('rule.addRule')}
		</button>
	</div>

	<RuleLibrary
		onEdit={(rule) => (editingRule = rule)}
		onDelete={(rule) => (deletingRule = rule)}
	/>
</div>

<!-- Add Rule Modal -->
{#if showAddRule}
	<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
		<div class="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-2xl w-full mx-4 max-h-[90vh] overflow-auto">
			<div class="p-6">
				<h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-6">{i18n.t('rule.addNew')}</h2>
				<RuleForm onSubmit={handleCreateRule} onCancel={() => (showAddRule = false)} />
			</div>
		</div>
	</div>
{/if}

<!-- Edit Rule Modal -->
{#if editingRule}
	<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
		<div class="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-2xl w-full mx-4 max-h-[90vh] overflow-auto">
			<div class="p-6">
				<h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-4">{i18n.t('rule.editRule')}</h2>
				<RuleForm
					initialValues={editingRule}
					onSubmit={handleUpdateRule}
					onCancel={() => (editingRule = null)}
				/>
			</div>
		</div>
	</div>
{/if}

<ConfirmDialog
	open={!!deletingRule}
	title={i18n.t('rule.deleteRule')}
	message={i18n.t('rule.deleteConfirm', { name: deletingRule?.name ?? '' })}
	confirmText={i18n.t('common.delete')}
	onConfirm={handleDeleteRule}
	onCancel={() => (deletingRule = null)}
/>
