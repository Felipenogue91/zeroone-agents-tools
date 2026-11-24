<script lang="ts">
	import type { ClaudeSettings, SandboxSettings, SandboxNetworkSettings } from '$lib/types';
	import { i18n } from '$lib/i18n';
	import { Save, Plus, X } from 'lucide-svelte';
	import SandboxNetworkEditor from './SandboxNetworkEditor.svelte';

	type Props = {
		settings: ClaudeSettings;
		onsave: (settings: ClaudeSettings) => void;
	};

	let { settings, onsave }: Props = $props();

	let enabled = $state<boolean | undefined>(settings.sandbox?.enabled);
	let autoAllowBashIfSandboxed = $state<boolean | undefined>(
		settings.sandbox?.autoAllowBashIfSandboxed
	);
	let allowUnsandboxedCommands = $state<boolean | undefined>(
		settings.sandbox?.allowUnsandboxedCommands
	);
	let enableWeakerNestedSandbox = $state<boolean | undefined>(
		settings.sandbox?.enableWeakerNestedSandbox
	);
	let excludedCommands = $state<string[]>([...(settings.sandbox?.excludedCommands ?? [])]);
	let network = $state<SandboxNetworkSettings>(settings.sandbox?.network ?? {});
	let newCommand = $state('');

	// Reset local state when settings prop changes
	$effect(() => {
		enabled = settings.sandbox?.enabled;
		autoAllowBashIfSandboxed = settings.sandbox?.autoAllowBashIfSandboxed;
		allowUnsandboxedCommands = settings.sandbox?.allowUnsandboxedCommands;
		enableWeakerNestedSandbox = settings.sandbox?.enableWeakerNestedSandbox;
		excludedCommands = [...(settings.sandbox?.excludedCommands ?? [])];
		network = settings.sandbox?.network ?? {};
	});

	function handleSave() {
		// Build sandbox object, only including set fields
		const hasAnyValue =
			enabled !== undefined ||
			autoAllowBashIfSandboxed !== undefined ||
			allowUnsandboxedCommands !== undefined ||
			enableWeakerNestedSandbox !== undefined ||
			excludedCommands.length > 0 ||
			Object.keys(network).length > 0;

		let sandbox: SandboxSettings | undefined;
		if (hasAnyValue) {
			sandbox = {};
			if (enabled !== undefined) sandbox.enabled = enabled;
			if (autoAllowBashIfSandboxed !== undefined)
				sandbox.autoAllowBashIfSandboxed = autoAllowBashIfSandboxed;
			if (allowUnsandboxedCommands !== undefined)
				sandbox.allowUnsandboxedCommands = allowUnsandboxedCommands;
			if (enableWeakerNestedSandbox !== undefined)
				sandbox.enableWeakerNestedSandbox = enableWeakerNestedSandbox;
			if (excludedCommands.length > 0) sandbox.excludedCommands = excludedCommands;
			if (Object.keys(network).length > 0) sandbox.network = network;
		}

		onsave({
			...settings,
			sandbox
		});
	}

	function handleToggle(field: 'enabled' | 'autoAllowBashIfSandboxed' | 'allowUnsandboxedCommands' | 'enableWeakerNestedSandbox') {
		if (field === 'enabled') {
			enabled = enabled === undefined ? true : enabled ? false : undefined;
		} else if (field === 'autoAllowBashIfSandboxed') {
			autoAllowBashIfSandboxed =
				autoAllowBashIfSandboxed === undefined
					? true
					: autoAllowBashIfSandboxed
						? false
						: undefined;
		} else if (field === 'allowUnsandboxedCommands') {
			allowUnsandboxedCommands =
				allowUnsandboxedCommands === undefined
					? true
					: allowUnsandboxedCommands
						? false
						: undefined;
		} else if (field === 'enableWeakerNestedSandbox') {
			enableWeakerNestedSandbox =
				enableWeakerNestedSandbox === undefined
					? true
					: enableWeakerNestedSandbox
						? false
						: undefined;
		}
	}

	function getTriStateLabel(value: boolean | undefined): string {
		if (value === undefined) return i18n.t('settingsComp.notSet');
		return value ? i18n.t('settingsComp.enabledLabel') : i18n.t('settingsComp.disabledLabel');
	}

	function getTriStateColor(value: boolean | undefined): string {
		if (value === undefined)
			return 'bg-gray-200 dark:bg-gray-600';
		return value ? 'bg-primary-600' : 'bg-red-400 dark:bg-red-600';
	}

	function getTriStatePosition(value: boolean | undefined): string {
		if (value === undefined) return 'translate-x-1';
		return value ? 'translate-x-6' : 'translate-x-3.5';
	}

	function addCommand() {
		const trimmed = newCommand.trim();
		if (trimmed && !excludedCommands.includes(trimmed)) {
			excludedCommands = [...excludedCommands, trimmed];
			newCommand = '';
		}
	}

	function removeCommand(index: number) {
		excludedCommands = excludedCommands.filter((_, i) => i !== index);
	}

	function handleNetworkChange(updated: SandboxNetworkSettings) {
		network = updated;
	}

	const toggleFields = $derived([
		{
			key: 'enabled' as const,
			label: i18n.t('settingsComp.sandboxEnabled'),
			description: i18n.t('settingsComp.sandboxEnabledDesc')
		},
		{
			key: 'autoAllowBashIfSandboxed' as const,
			label: i18n.t('settingsComp.autoAllowBash'),
			description: i18n.t('settingsComp.autoAllowBashDesc')
		},
		{
			key: 'allowUnsandboxedCommands' as const,
			label: i18n.t('settingsComp.allowUnsandboxed'),
			description: i18n.t('settingsComp.allowUnsandboxedDesc')
		},
		{
			key: 'enableWeakerNestedSandbox' as const,
			label: i18n.t('settingsComp.weakerNestedSandbox'),
			description: i18n.t('settingsComp.weakerNestedSandboxDesc')
		}
	]);

	function getFieldValue(key: (typeof toggleFields)[number]['key']): boolean | undefined {
		if (key === 'enabled') return enabled;
		if (key === 'autoAllowBashIfSandboxed') return autoAllowBashIfSandboxed;
		if (key === 'allowUnsandboxedCommands') return allowUnsandboxedCommands;
		if (key === 'enableWeakerNestedSandbox') return enableWeakerNestedSandbox;
		return undefined;
	}
</script>

<div class="space-y-6">
	<!-- General Settings -->
	<div class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-5">
		<h3 class="text-base font-semibold text-gray-900 dark:text-white mb-1">{i18n.t('settingsComp.general')}</h3>
		<p class="text-xs text-gray-500 dark:text-gray-400 mb-4">
			{i18n.t('settingsComp.sandboxGeneralDesc')}
		</p>

		<div class="space-y-4">
			{#each toggleFields as field}
				{@const value = getFieldValue(field.key)}
				<div class="flex items-center justify-between">
					<div>
						<label class="text-sm font-medium text-gray-700 dark:text-gray-300">
							{field.label}
						</label>
						<p class="text-xs text-gray-500 dark:text-gray-400">
							{field.description}
						</p>
					</div>
					<div class="flex items-center gap-2">
						<span class="text-xs text-gray-500 dark:text-gray-400 min-w-[60px] text-right">
							{getTriStateLabel(value)}
						</span>
						<button
							onclick={() => handleToggle(field.key)}
							class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors {getTriStateColor(value)}"
							title={i18n.t('settingsComp.triStateCycleTitle')}
						>
							<span
								class="inline-block h-4 w-4 transform rounded-full bg-white transition-transform {getTriStatePosition(value)}"
							></span>
						</button>
					</div>
				</div>
			{/each}
		</div>
	</div>

	<!-- Excluded Commands -->
	<div class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-5">
		<h3 class="text-base font-semibold text-gray-900 dark:text-white mb-1">{i18n.t('settingsComp.excludedCommands')}</h3>
		<p class="text-xs text-gray-500 dark:text-gray-400 mb-4">
			{i18n.t('settingsComp.excludedCommandsDescPrefix')} <code>git</code>, <code>docker</code>)
		</p>

		<div class="flex gap-2 mb-3">
			<input
				type="text"
				bind:value={newCommand}
				placeholder={i18n.t('settingsComp.excludedCommandPlaceholder')}
				class="input text-sm flex-1"
				onkeydown={(e) => e.key === 'Enter' && addCommand()}
			/>
			<button
				onclick={addCommand}
				disabled={!newCommand.trim()}
				class="btn btn-ghost"
			>
				<Plus class="w-4 h-4" />
			</button>
		</div>

		{#if excludedCommands.length > 0}
			<div class="flex flex-wrap gap-2">
				{#each excludedCommands as cmd, i}
					<span
						class="inline-flex items-center gap-1 px-2.5 py-1 rounded-md text-xs font-medium bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300"
					>
						<code>{cmd}</code>
						<button
							onclick={() => removeCommand(i)}
							class="text-gray-400 hover:text-red-500"
						>
							<X class="w-3 h-3" />
						</button>
					</span>
				{/each}
			</div>
		{:else}
			<p class="text-xs text-gray-500 dark:text-gray-400 italic">
				{i18n.t('settingsComp.noExcludedCommands')}
			</p>
		{/if}
	</div>

	<!-- Network Configuration -->
	<div class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-5">
		<h3 class="text-base font-semibold text-gray-900 dark:text-white mb-1">{i18n.t('settingsComp.networkTitle')}</h3>
		<p class="text-xs text-gray-500 dark:text-gray-400 mb-4">
			{i18n.t('settingsComp.networkDesc')}
		</p>

		<SandboxNetworkEditor {network} onchange={handleNetworkChange} />
	</div>

	<!-- Save Button -->
	<div class="flex justify-end">
		<button onclick={handleSave} class="btn btn-primary">
			<Save class="w-4 h-4 mr-2" />
			{i18n.t('settingsComp.saveSandboxSettings')}
		</button>
	</div>
</div>
