<script lang="ts">
	import type { ClaudeSettings } from '$lib/types';
	import { i18n } from '$lib/i18n';
	import { Save } from 'lucide-svelte';

	type Props = {
		settings: ClaudeSettings;
		onsave: (settings: ClaudeSettings) => void;
	};

	let { settings, onsave }: Props = $props();

	const fields = $derived([
		{
			key: 'apiKeyHelper' as const,
			label: i18n.t('settingsComp.apiKeyHelper'),
			description: i18n.t('settingsComp.apiKeyHelperDesc'),
			placeholder: '/path/to/api-key-helper.sh'
		},
		{
			key: 'otelHeadersHelper' as const,
			label: i18n.t('settingsComp.otelHeadersHelper'),
			description: i18n.t('settingsComp.otelHeadersHelperDesc'),
			placeholder: '/path/to/otel-headers-helper.sh'
		},
		{
			key: 'awsAuthRefresh' as const,
			label: i18n.t('settingsComp.awsAuthRefresh'),
			description: i18n.t('settingsComp.awsAuthRefreshDesc'),
			placeholder: '/path/to/aws-auth-refresh.sh'
		},
		{
			key: 'awsCredentialExport' as const,
			label: i18n.t('settingsComp.awsCredentialExport'),
			description: i18n.t('settingsComp.awsCredentialExportDesc'),
			placeholder: '/path/to/aws-credential-export.sh'
		}
	]);

	let values = $state<Record<string, string>>({
		apiKeyHelper: settings.apiKeyHelper ?? '',
		otelHeadersHelper: settings.otelHeadersHelper ?? '',
		awsAuthRefresh: settings.awsAuthRefresh ?? '',
		awsCredentialExport: settings.awsCredentialExport ?? ''
	});

	$effect(() => {
		values = {
			apiKeyHelper: settings.apiKeyHelper ?? '',
			otelHeadersHelper: settings.otelHeadersHelper ?? '',
			awsAuthRefresh: settings.awsAuthRefresh ?? '',
			awsCredentialExport: settings.awsCredentialExport ?? ''
		};
	});

	function handleSave() {
		onsave({
			...settings,
			apiKeyHelper: values.apiKeyHelper.trim() || undefined,
			otelHeadersHelper: values.otelHeadersHelper.trim() || undefined,
			awsAuthRefresh: values.awsAuthRefresh.trim() || undefined,
			awsCredentialExport: values.awsCredentialExport.trim() || undefined
		});
	}
</script>

<div class="space-y-6">
	<div class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-5">
		<h3 class="text-base font-semibold text-gray-900 dark:text-white mb-1">{i18n.t('settingsComp.authHelpersTitle')}</h3>
		<p class="text-xs text-gray-500 dark:text-gray-400 mb-4">
			{i18n.t('settingsComp.authHelpersDesc')}
		</p>

		<div class="space-y-4">
			{#each fields as field}
				<div>
					<label for={field.key} class="text-sm font-medium text-gray-700 dark:text-gray-300">
						{field.label}
					</label>
					<p class="text-xs text-gray-500 dark:text-gray-400 mb-2">
						{field.description}
					</p>
					<input
						id={field.key}
						type="text"
						bind:value={values[field.key]}
						placeholder={field.placeholder}
						class="input w-full"
					/>
				</div>
			{/each}
		</div>
	</div>

	<div class="flex justify-end">
		<button onclick={handleSave} class="btn btn-primary">
			<Save class="w-4 h-4 mr-2" />
			{i18n.t('settingsComp.saveAuthHelpers')}
		</button>
	</div>
</div>
