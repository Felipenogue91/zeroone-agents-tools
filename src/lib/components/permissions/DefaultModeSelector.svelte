<script lang="ts">
	import { i18n } from '$lib/i18n';

	type Props = {
		value: string | undefined;
		onchange: (mode: string | null) => void;
	};

	let { value, onchange }: Props = $props();

	const modes = $derived([
		{
			value: '',
			label: i18n.t('permComp.modeNotSet'),
			description: i18n.t('permComp.modeNotSetDesc')
		},
		{
			value: 'default',
			label: i18n.t('permComp.modeDefault'),
			description: i18n.t('permComp.modeDefaultDesc')
		},
		{
			value: 'allowEdits',
			label: i18n.t('permComp.modeAllowEdits'),
			description: i18n.t('permComp.modeAllowEditsDesc')
		},
		{
			value: 'bypassPermissions',
			label: i18n.t('permComp.modeBypass'),
			description: i18n.t('permComp.modeBypassDesc')
		}
	]);

	function handleChange(e: Event) {
		const target = e.target as HTMLSelectElement;
		onchange(target.value || null);
	}
</script>

<div class="flex items-center gap-3">
	<label class="text-sm font-medium text-gray-700 dark:text-gray-300 whitespace-nowrap">
		{i18n.t('permComp.defaultMode')}
	</label>
	<select value={value ?? ''} onchange={handleChange} class="input text-sm flex-1 max-w-xs">
		{#each modes as mode}
			<option value={mode.value}>{mode.label}</option>
		{/each}
	</select>
	{#if value}
		<span class="text-xs text-gray-500 dark:text-gray-400">
			{modes.find((m) => m.value === value)?.description ?? ''}
		</span>
	{/if}
</div>
