<script lang="ts">
	import { ScopedSettingsWrapper } from '$lib/components/settings';
	import { FileSuggestionEditor } from '$lib/components/file-suggestion';
	import { claudeSettingsLibrary } from '$lib/stores';
	import { i18n } from '$lib/i18n';
	import type { ClaudeSettingsScope } from '$lib/types';

	function getSettingCount(scope: ClaudeSettingsScope): number {
		if (!claudeSettingsLibrary.settings) return 0;
		const s =
			scope === 'user'
				? claudeSettingsLibrary.settings.user
				: scope === 'project'
					? claudeSettingsLibrary.settings.project
					: claudeSettingsLibrary.settings.local;
		if (!s) return 0;
		let count = 0;
		if (s.fileSuggestionCommand) count++;
		return count;
	}
</script>

<ScopedSettingsWrapper {getSettingCount}>
	{#snippet children({ settings, save })}
		<FileSuggestionEditor
			{settings}
			onsave={(s) => save(s, i18n.t('settingsComp.filesSaved'), i18n.t('settingsComp.filesSaveFailed'))}
		/>
	{/snippet}
</ScopedSettingsWrapper>
