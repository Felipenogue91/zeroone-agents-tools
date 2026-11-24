<script lang="ts">
	import { containerLibrary, notifications } from '$lib/stores';
	import { i18n } from '$lib/i18n';
	import type { Container } from '$lib/types';
	import ContainerStatusBadge from './ContainerStatus.svelte';
	import ContainerActions from './ContainerActions.svelte';
	import ContainerLogs from './ContainerLogs.svelte';
	import ContainerStats from './ContainerStats.svelte';
	import { X, Terminal, Copy, Check } from 'lucide-svelte';

	let { container, onClose }: {
		container: Container;
		onClose: () => void;
	} = $props();

	let activeTab = $state<'overview' | 'logs' | 'stats' | 'exec'>('overview');
	let execCommand = $state('');
	let execOutput = $state('');
	let execRunning = $state(false);
	let actionInProgress = $state(false);

	const status = $derived(containerLibrary.getStatus(container.id));
	const dockerStatus = $derived(status?.dockerStatus || 'not_created');

	const tabs = ['overview', 'logs', 'stats', 'exec'] as const;
	const tabLabels = $derived<Record<(typeof tabs)[number], string>>({
		overview: i18n.t('containerComp.tabOverview'),
		logs: i18n.t('containerComp.tabLogs'),
		stats: i18n.t('containerComp.tabStats'),
		exec: i18n.t('containerComp.tabExec')
	});

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onClose();
	}

	function handleTabKeydown(e: KeyboardEvent) {
		const currentIndex = tabs.indexOf(activeTab);
		let nextIndex = -1;
		if (e.key === 'ArrowRight') {
			e.preventDefault();
			nextIndex = (currentIndex + 1) % tabs.length;
		} else if (e.key === 'ArrowLeft') {
			e.preventDefault();
			nextIndex = (currentIndex - 1 + tabs.length) % tabs.length;
		} else if (e.key === 'Home') {
			e.preventDefault();
			nextIndex = 0;
		} else if (e.key === 'End') {
			e.preventDefault();
			nextIndex = tabs.length - 1;
		}
		if (nextIndex >= 0) {
			activeTab = tabs[nextIndex];
		}
	}

	async function handleExec() {
		if (!execCommand.trim() || execRunning) return;
		execRunning = true;
		const cmd = execCommand;
		execCommand = '';
		try {
			const result = await containerLibrary.exec(container.id, ['sh', '-c', cmd]);
			execOutput += `$ ${cmd}\n${result.stdout}${result.stderr}\n`;
		} catch (e) {
			execOutput += `$ ${cmd}\n${i18n.t('containerComp.execError', { error: String(e) })}\n`;
		} finally {
			execRunning = false;
		}
	}

	async function withAction(fn: () => Promise<void>) {
		if (actionInProgress) return;
		actionInProgress = true;
		try { await fn(); } finally { actionInProgress = false; }
	}

	async function handleStart() { await withAction(async () => { await containerLibrary.startContainer(container.id); notifications.success(i18n.t('containerComp.started')); }); }
	async function handleStop() { await withAction(async () => { await containerLibrary.stopContainer(container.id); notifications.success(i18n.t('containerComp.stopped')); }); }
	async function handleRestart() { await withAction(async () => { await containerLibrary.restartContainer(container.id); notifications.success(i18n.t('containerComp.restarted')); }); }
	async function handleRemove() { await withAction(async () => { await containerLibrary.removeContainer(container.id); notifications.success(i18n.t('containerComp.removed')); }); }
	async function handleBuild() { await withAction(async () => { await containerLibrary.buildImage(container.id); notifications.success(i18n.t('containerComp.built')); }); }
	async function handleRecreate() { await withAction(async () => { await containerLibrary.removeContainer(container.id); await containerLibrary.startContainer(container.id); notifications.success(i18n.t('containerComp.recreated')); }); }

	let copiedCommand = $state<string | null>(null);
	const containerName = $derived(`cctm-${container.name.toLowerCase().replace(/\s+/g, '-')}`);
	const execCommand_shell = $derived(`docker exec -it ${containerName} bash`);
	const execCommand_claude = $derived(`docker exec -it ${containerName} claude`);

	async function copyToClipboard(text: string, label: string) {
		await navigator.clipboard.writeText(text);
		copiedCommand = label;
		setTimeout(() => copiedCommand = null, 2000);
	}
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50" role="dialog" aria-modal="true" onkeydown={handleKeydown}
	onclick={(e) => e.target === e.currentTarget && onClose()}>
	<div class="bg-white dark:bg-gray-800 rounded-xl shadow-xl w-full max-w-4xl mx-4 max-h-[85vh] flex flex-col">
		<div class="flex items-center justify-between p-4 border-b border-gray-200 dark:border-gray-700">
			<div class="flex items-center gap-3">
				<span class="text-2xl">{container.icon || '📦'}</span>
				<div>
					<div class="flex items-center gap-2">
						<h2 class="text-lg font-semibold text-gray-900 dark:text-white">{container.name}</h2>
						<ContainerStatusBadge status={dockerStatus} />
					</div>
					{#if container.description}
						<p class="text-sm text-gray-500 dark:text-gray-400">{container.description}</p>
					{/if}
				</div>
			</div>
			<div class="flex items-center gap-2">
				<ContainerActions status={dockerStatus}
					disabled={actionInProgress}
					onBuild={container.dockerfile ? handleBuild : undefined}
					onStart={handleStart} onStop={handleStop} onRestart={handleRestart} onRemove={handleRemove}
					onRecreate={container.dockerContainerId ? handleRecreate : undefined} />
				<button onclick={onClose} class="p-1.5 rounded-lg text-gray-400 hover:text-gray-600 hover:bg-gray-100 dark:hover:text-gray-300 dark:hover:bg-gray-700 transition-colors" aria-label={i18n.t('common.close')}>
					<X class="w-5 h-5" aria-hidden="true" />
				</button>
			</div>
		</div>

		<div class="flex border-b border-gray-200 dark:border-gray-700" role="tablist" aria-label={i18n.t('containerComp.containerDetails')}>
			{#each tabs as tab}
				{@const isActive = activeTab === tab}
				<button
					role="tab"
					aria-selected={isActive}
					aria-controls="container-tabpanel"
					id="container-tab-{tab}"
					tabindex={isActive ? 0 : -1}
					onclick={() => activeTab = tab}
					onkeydown={handleTabKeydown}
					class="px-4 py-2 text-sm font-medium border-b-2 transition-colors
						{isActive ? 'border-primary-500 text-primary-600 dark:text-primary-400' : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200'}">
					{tabLabels[tab]}
				</button>
			{/each}
		</div>

		<div id="container-tabpanel" role="tabpanel" aria-labelledby="container-tab-{activeTab}" class="flex-1 overflow-auto">
			{#if activeTab === 'overview'}
				<div class="p-4 space-y-3">
					<div class="grid grid-cols-2 gap-4 text-sm">
						<div><span class="text-gray-500 dark:text-gray-400">{i18n.t('containerComp.typeLabel')}</span> <span class="text-gray-900 dark:text-white">{container.containerType}</span></div>
						<div><span class="text-gray-500 dark:text-gray-400">{i18n.t('containerComp.imageLabel')}</span> <span class="text-gray-900 dark:text-white">{container.image || i18n.t('containerComp.na')}</span></div>
						<div><span class="text-gray-500 dark:text-gray-400">{i18n.t('containerComp.workingDirLabel')}</span> <span class="text-gray-900 dark:text-white">{container.workingDir || i18n.t('containerComp.na')}</span></div>
						<div class="min-w-0"><span class="text-gray-500 dark:text-gray-400">{i18n.t('containerComp.dockerIdLabel')}</span> <span class="text-gray-900 dark:text-white font-mono text-xs break-all">{container.dockerContainerId || i18n.t('containerComp.na')}</span></div>
					</div>
					{#if container.ports && container.ports.length > 0}
						<div>
							<p class="text-sm font-medium text-gray-500 dark:text-gray-400 mb-1">{i18n.t('containerComp.ports')}</p>
							<div class="flex flex-wrap gap-2">
								{#each container.ports as port}
									<span class="text-xs px-2 py-1 bg-gray-100 dark:bg-gray-700 rounded">{port.hostPort}:{port.containerPort}/{port.protocol || 'tcp'}</span>
								{/each}
							</div>
						</div>
					{/if}
					{#if container.env && Object.keys(container.env).length > 0}
						<div>
							<p class="text-sm font-medium text-gray-500 dark:text-gray-400 mb-1">{i18n.t('containerComp.environment')}</p>
							<div class="font-mono text-xs space-y-0.5">
								{#each Object.entries(container.env) as [key, value]}
									<div><span class="text-blue-600 dark:text-blue-400">{key}</span>=<span class="text-gray-600 dark:text-gray-400">{value}</span></div>
								{/each}
							</div>
						</div>
					{/if}
					{#if dockerStatus === 'running'}
						<div>
							<p class="text-sm font-medium text-gray-500 dark:text-gray-400 mb-2">{i18n.t('containerComp.connectFromTerminal')}</p>
							<div class="space-y-2">
								<div class="flex items-center gap-2 bg-gray-950 rounded-lg px-3 py-2">
									<code class="text-xs text-gray-300 flex-1 select-all truncate">{execCommand_shell}</code>
									<button onclick={() => copyToClipboard(execCommand_shell, 'shell')}
										class="p-1 rounded text-gray-500 hover:text-white transition-colors shrink-0" title={i18n.t('containerComp.copyToClipboard')}>
										{#if copiedCommand === 'shell'}
											<Check class="w-3.5 h-3.5 text-green-400" />
										{:else}
											<Copy class="w-3.5 h-3.5" />
										{/if}
									</button>
								</div>
								<div class="flex items-center gap-2 bg-gray-950 rounded-lg px-3 py-2">
									<code class="text-xs text-gray-300 flex-1 select-all truncate">{execCommand_claude}</code>
									<button onclick={() => copyToClipboard(execCommand_claude, 'claude')}
										class="p-1 rounded text-gray-500 hover:text-white transition-colors shrink-0" title={i18n.t('containerComp.copyToClipboard')}>
										{#if copiedCommand === 'claude'}
											<Check class="w-3.5 h-3.5 text-green-400" />
										{:else}
											<Copy class="w-3.5 h-3.5" />
										{/if}
									</button>
								</div>
							</div>
						</div>
					{/if}
				</div>
			{:else if activeTab === 'logs'}
				<div class="h-96">
					<ContainerLogs containerId={container.id} />
				</div>
			{:else if activeTab === 'stats'}
				<ContainerStats containerId={container.id} />
			{:else if activeTab === 'exec'}
				<div class="p-4 flex flex-col h-96">
					<div class="flex-1 bg-gray-950 rounded-lg p-3 font-mono text-xs text-gray-300 overflow-auto mb-2 whitespace-pre break-all">{#if execOutput}{execOutput}{:else}<span class="text-gray-600 italic">{i18n.t('containerComp.runCommandPlaceholder')}</span>{/if}</div>
					<div class="flex gap-2">
						<input type="text" bind:value={execCommand} placeholder={i18n.t('containerComp.enterCommandPlaceholder')}
							disabled={execRunning || dockerStatus !== 'running'}
							onkeydown={(e) => e.key === 'Enter' && handleExec()}
							class="input flex-1 min-w-0 font-mono" />
						<button onclick={handleExec} disabled={execRunning || dockerStatus !== 'running' || !execCommand.trim()}
							class="btn btn-primary shrink-0">
							<Terminal class="w-4 h-4" aria-hidden="true" />
						</button>
					</div>
					{#if dockerStatus !== 'running'}
						<p class="text-xs text-gray-500 mt-1">{i18n.t('containerComp.mustBeRunningToExec')}</p>
					{/if}
				</div>
			{/if}
		</div>
	</div>
</div>
