<script lang="ts">
	import type { CreateStatusLineRequest, StatusLine } from '$lib/types';
	import { i18n } from '$lib/i18n';

	type Props = {
		initialValues?: StatusLine | null;
		onSubmit: (request: CreateStatusLineRequest) => void;
		onCancel: () => void;
	};

	let { initialValues, onSubmit, onCancel }: Props = $props();

	let name = $state(initialValues?.name ?? '');
	let description = $state(initialValues?.description ?? '');
	let rawCommand = $state(initialValues?.rawCommand ?? '');
	let padding = $state(initialValues?.padding ?? 0);

	function handleSubmit() {
		if (!name.trim() || !rawCommand.trim()) return;
		onSubmit({
			name: name.trim(),
			description: description.trim() || null,
			statuslineType: 'raw',
			rawCommand: rawCommand.trim(),
			padding
		});
	}
</script>

<form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }} class="space-y-4">
	<div>
		<label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{i18n.t('common.name')}</label>
		<input
			type="text"
			bind:value={name}
			placeholder={i18n.t('statuslineComp.namePlaceholder')}
			required
			class="w-full px-3 py-2 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg text-sm text-gray-900 dark:text-white placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-primary-500"
		/>
	</div>

	<div>
		<label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{i18n.t('common.description')}</label>
		<input
			type="text"
			bind:value={description}
			placeholder={i18n.t('statuslineComp.descPlaceholder')}
			class="w-full px-3 py-2 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg text-sm text-gray-900 dark:text-white placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-primary-500"
		/>
	</div>

	<div>
		<label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{i18n.t('statuslineComp.command')}</label>
		<input
			type="text"
			bind:value={rawCommand}
			placeholder={i18n.t('statuslineComp.commandPlaceholder')}
			required
			class="w-full px-3 py-2 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg text-sm text-gray-900 dark:text-white font-mono placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-primary-500"
		/>
		<p class="text-xs text-gray-400 dark:text-gray-500 mt-1">
			{i18n.t('statuslineComp.commandHelp')}
		</p>
	</div>

	<div>
		<label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{i18n.t('statuslineComp.padding')}</label>
		<input
			type="number"
			min="0"
			max="10"
			bind:value={padding}
			class="w-24 px-3 py-2 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg text-sm text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary-500"
		/>
	</div>

	<div class="flex justify-end gap-3 pt-2">
		<button type="button" onclick={onCancel} class="btn btn-secondary">
			{i18n.t('common.cancel')}
		</button>
		<button type="submit" class="btn btn-primary" disabled={!name.trim() || !rawCommand.trim()}>
			{initialValues ? i18n.t('statuslineComp.update') : i18n.t('statuslineComp.create')}
		</button>
	</div>
</form>
