<script lang="ts">
	import type { CreateDockerHostRequest, DockerHostType } from '$lib/types';
	import { i18n } from '$lib/i18n';

	let { onSubmit, onCancel }: {
		onSubmit: (values: CreateDockerHostRequest) => void;
		onCancel: () => void;
	} = $props();

	let name = $state('');
	let hostType = $state<DockerHostType>('local');
	let connectionUri = $state('');
	let sshKeyPath = $state('');

	function handleSubmit(e: Event) {
		e.preventDefault();
		onSubmit({
			name,
			hostType,
			connectionUri: connectionUri || undefined,
			sshKeyPath: sshKeyPath || undefined
		});
	}
</script>

<form onsubmit={handleSubmit} class="space-y-4">
	<div>
		<label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{i18n.t('containerComp.nameRequired')}</label>
		<input type="text" bind:value={name} required
			class="input" />
	</div>
	<div>
		<label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{i18n.t('containerComp.hostType')}</label>
		<select bind:value={hostType}
			class="input">
			<option value="local">{i18n.t('containerComp.hostLocal')}</option>
			<option value="ssh">SSH</option>
			<option value="tcp">TCP</option>
		</select>
	</div>
	{#if hostType !== 'local'}
		<div>
			<label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{i18n.t('containerComp.connectionUriRequired')}</label>
			<input type="text" bind:value={connectionUri} required placeholder={hostType === 'ssh' ? 'ssh://user@host' : 'tcp://host:2376'}
				class="input" />
		</div>
		{#if hostType === 'ssh'}
			<div>
				<label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{i18n.t('containerComp.sshKeyPath')}</label>
				<input type="text" bind:value={sshKeyPath} placeholder="~/.ssh/id_rsa"
					class="input" />
			</div>
		{/if}
	{/if}
	<div class="flex justify-end gap-3 pt-2">
		<button type="button" onclick={onCancel}
			class="btn btn-secondary">
			{i18n.t('common.cancel')}
		</button>
		<button type="submit" disabled={!name || (hostType !== 'local' && !connectionUri)}
			class="btn btn-primary">
			{i18n.t('containers.addHost')}
		</button>
	</div>
</form>
