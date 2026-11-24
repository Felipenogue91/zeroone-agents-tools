<script lang="ts">
	import type { ClaudeSettings } from '$lib/types';
	import { i18n } from '$lib/i18n';
	import { Save, X } from 'lucide-svelte';

	type Props = {
		settings: ClaudeSettings;
		onsave: (settings: ClaudeSettings) => void;
	};

	let { settings, onsave }: Props = $props();

	const DEFAULT_COMMIT = 'Co-Authored-By: Claude <noreply@anthropic.com>';
	const DEFAULT_PR = 'Generated with [Claude Code](https://claude.ai/code)';

	let commitText = $state(settings.attributionCommit ?? '');
	let prText = $state(settings.attributionPr ?? '');
	let commitHasValue = $state(settings.attributionCommit !== undefined);
	let prHasValue = $state(settings.attributionPr !== undefined);

	// Reset local state when settings prop changes
	$effect(() => {
		commitText = settings.attributionCommit ?? '';
		prText = settings.attributionPr ?? '';
		commitHasValue = settings.attributionCommit !== undefined;
		prHasValue = settings.attributionPr !== undefined;
	});

	function handleSave() {
		onsave({
			...settings,
			attributionCommit: commitHasValue ? commitText : undefined,
			attributionPr: prHasValue ? prText : undefined
		});
	}

	function clearCommit() {
		commitHasValue = true;
		commitText = '';
	}

	function clearPr() {
		prHasValue = true;
		prText = '';
	}

	function unsetCommit() {
		commitHasValue = false;
		commitText = '';
	}

	function unsetPr() {
		prHasValue = false;
		prText = '';
	}
</script>

<div class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-5">
	<h3 class="text-base font-semibold text-gray-900 dark:text-white mb-1">{i18n.t('settingsComp.attributionTitle')}</h3>
	<p class="text-xs text-gray-500 dark:text-gray-400 mb-4">
		{i18n.t('settingsComp.attributionDesc')}
	</p>

	<div class="space-y-4">
		<!-- Commit Attribution -->
		<div>
			<div class="flex items-center justify-between mb-1">
				<label
					for="commit-attribution"
					class="text-sm font-medium text-gray-700 dark:text-gray-300"
				>
					{i18n.t('settingsComp.commitAttribution')}
				</label>
				<div class="flex gap-1">
					{#if commitHasValue}
						<button
							onclick={unsetCommit}
							class="text-xs text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
							title={i18n.t('settingsComp.unsetUseDefault')}
						>
							<X class="w-3.5 h-3.5" />
						</button>
					{:else}
						<button
							onclick={clearCommit}
							class="text-xs text-primary-600 hover:text-primary-700 dark:text-primary-400"
						>
							{i18n.t('settingsComp.setToEmptyHide')}
						</button>
					{/if}
				</div>
			</div>
			{#if commitHasValue}
				<textarea
					id="commit-attribution"
					bind:value={commitText}
					placeholder={i18n.t('settingsComp.commitPlaceholder')}
					rows={2}
					class="input text-sm w-full resize-y"
				></textarea>
				{#if commitText === ''}
					<p class="text-xs text-amber-600 dark:text-amber-400 mt-1">
						{i18n.t('settingsComp.commitHiddenWarn')}
					</p>
				{/if}
			{:else}
				<p class="text-xs text-gray-500 dark:text-gray-400 italic">
					{i18n.t('settingsComp.notSetUsingDefault', { value: DEFAULT_COMMIT })}
				</p>
			{/if}
		</div>

		<!-- PR Attribution -->
		<div>
			<div class="flex items-center justify-between mb-1">
				<label
					for="pr-attribution"
					class="text-sm font-medium text-gray-700 dark:text-gray-300"
				>
					{i18n.t('settingsComp.prAttribution')}
				</label>
				<div class="flex gap-1">
					{#if prHasValue}
						<button
							onclick={unsetPr}
							class="text-xs text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
							title={i18n.t('settingsComp.unsetUseDefault')}
						>
							<X class="w-3.5 h-3.5" />
						</button>
					{:else}
						<button
							onclick={clearPr}
							class="text-xs text-primary-600 hover:text-primary-700 dark:text-primary-400"
						>
							{i18n.t('settingsComp.setToEmptyHide')}
						</button>
					{/if}
				</div>
			</div>
			{#if prHasValue}
				<textarea
					id="pr-attribution"
					bind:value={prText}
					placeholder={i18n.t('settingsComp.prPlaceholder')}
					rows={2}
					class="input text-sm w-full resize-y"
				></textarea>
				{#if prText === ''}
					<p class="text-xs text-amber-600 dark:text-amber-400 mt-1">
						{i18n.t('settingsComp.prHiddenWarn')}
					</p>
				{/if}
			{:else}
				<p class="text-xs text-gray-500 dark:text-gray-400 italic">
					{i18n.t('settingsComp.notSetUsingDefault', { value: DEFAULT_PR })}
				</p>
			{/if}
		</div>

		<!-- Preview -->
		{#if commitHasValue || prHasValue}
			<div class="border-t border-gray-200 dark:border-gray-700 pt-4">
				<h4 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">{i18n.t('common.preview')}</h4>
				<div class="space-y-2">
					{#if commitHasValue}
						<div class="bg-gray-50 dark:bg-gray-900/50 rounded p-3">
							<p class="text-xs text-gray-500 dark:text-gray-400 mb-1">{i18n.t('settingsComp.commitFooterLabel')}</p>
							<pre class="text-xs text-gray-700 dark:text-gray-300 whitespace-pre-wrap">{commitText || i18n.t('settingsComp.hiddenEmptyString')}</pre>
						</div>
					{/if}
					{#if prHasValue}
						<div class="bg-gray-50 dark:bg-gray-900/50 rounded p-3">
							<p class="text-xs text-gray-500 dark:text-gray-400 mb-1">{i18n.t('settingsComp.prFooterLabel')}</p>
							<pre class="text-xs text-gray-700 dark:text-gray-300 whitespace-pre-wrap">{prText || i18n.t('settingsComp.hiddenEmptyString')}</pre>
						</div>
					{/if}
				</div>
			</div>
		{/if}
	</div>

	<div class="mt-5 flex justify-end">
		<button onclick={handleSave} class="btn btn-primary">
			<Save class="w-4 h-4 mr-2" />
			{i18n.t('settingsComp.saveAttribution')}
		</button>
	</div>
</div>
