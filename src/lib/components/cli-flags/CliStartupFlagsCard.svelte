<script lang="ts">
	import { Copy, Check, Terminal, Clock } from 'lucide-svelte';
	import { i18n } from '$lib/i18n';

	interface CliFlag {
		flag: string;
		arg?: string;
		description: string;
		example: string;
	}

	const flags = $derived<CliFlag[]>([
		{ flag: '--agent', arg: '<name>', description: i18n.t('settingsComp.cliFlagAgentDesc'), example: 'claude --agent code-reviewer' },
		{ flag: '--baremode', description: i18n.t('settingsComp.cliFlagBaremodeDesc'), example: 'claude --baremode' },
		{ flag: '--system-prompt', arg: '<text>', description: i18n.t('settingsComp.cliFlagSystemPromptDesc'), example: 'claude --system-prompt "You are a Go expert"' },
		{ flag: '--append-system-prompt', arg: '<text>', description: i18n.t('settingsComp.cliFlagAppendSystemPromptDesc'), example: 'claude --append-system-prompt "Always use TypeScript"' },
		{ flag: '--permissions', arg: '<mode>', description: i18n.t('settingsComp.cliFlagPermissionsDesc'), example: 'claude --permissions plan' },
		{ flag: '--allowedTools', arg: '<tools>', description: i18n.t('settingsComp.cliFlagAllowedToolsDesc'), example: 'claude --allowedTools "Read,Grep,Glob"' },
		{ flag: '--disallowedTools', arg: '<tools>', description: i18n.t('settingsComp.cliFlagDisallowedToolsDesc'), example: 'claude --disallowedTools "Bash,Write"' },
		{ flag: '--model', arg: '<id>', description: i18n.t('settingsComp.cliFlagModelDesc'), example: 'claude --model claude-opus-4-6' },
		{ flag: '--max-turns', arg: '<n>', description: i18n.t('settingsComp.cliFlagMaxTurnsDesc'), example: 'claude --max-turns 10' },
	]);

	// Command builder state
	let selectedFlags = $state<Record<string, boolean>>({});
	let flagValues = $state<Record<string, string>>({});
	let copied = $state(false);

	const builtCommand = $derived.by(() => {
		const parts = ['claude'];
		for (const f of flags) {
			if (selectedFlags[f.flag]) {
				if (f.arg && flagValues[f.flag]) {
					parts.push(`${f.flag} ${flagValues[f.flag]}`);
				} else if (!f.arg) {
					parts.push(f.flag);
				}
			}
		}
		return parts.join(' ');
	});

	const hasSelection = $derived(Object.values(selectedFlags).some(Boolean));

	async function copyCommand() {
		try {
			await navigator.clipboard.writeText(builtCommand);
			copied = true;
			setTimeout(() => (copied = false), 2000);
		} catch {
			// Clipboard API may not be available
		}
	}
</script>

<div class="space-y-6">
	<!-- CLI Flags Reference -->
	<div class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-5">
		<div class="flex items-center gap-2 mb-1">
			<Terminal class="w-4 h-4 text-gray-500 dark:text-gray-400" />
			<h3 class="text-base font-semibold text-gray-900 dark:text-white">{i18n.t('settingsComp.cliStartupFlags')}</h3>
		</div>
		<p class="text-xs text-gray-500 dark:text-gray-400 mb-4">
			{i18n.t('settingsComp.cliStartupFlagsDesc')}
		</p>

		<div class="space-y-2">
			{#each flags as f}
				<label class="flex items-start gap-3 p-2.5 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700/50 cursor-pointer transition-colors">
					<input
						type="checkbox"
						bind:checked={selectedFlags[f.flag]}
						class="mt-1 rounded border-gray-300 dark:border-gray-600 text-primary-600 focus:ring-primary-500"
					/>
					<div class="flex-1 min-w-0">
						<div class="flex items-baseline gap-2">
							<code class="text-sm font-semibold text-gray-800 dark:text-gray-200">{f.flag}</code>
							{#if f.arg}
								<code class="text-xs text-gray-500 dark:text-gray-400">{f.arg}</code>
							{/if}
						</div>
						<p class="text-xs text-gray-500 dark:text-gray-400 mt-0.5">{f.description}</p>
						{#if selectedFlags[f.flag] && f.arg}
							<input
								type="text"
								bind:value={flagValues[f.flag]}
								placeholder={f.example.split(f.flag + ' ')[1] ?? ''}
								class="input text-xs mt-1.5 w-full max-w-xs font-mono"
							/>
						{/if}
					</div>
				</label>
			{/each}
		</div>

		{#if hasSelection}
			<div class="mt-4 bg-gray-50 dark:bg-gray-900/50 rounded-lg p-3">
				<div class="flex items-center justify-between mb-1">
					<p class="text-xs font-medium text-gray-500 dark:text-gray-400">{i18n.t('settingsComp.commandLabel')}</p>
					<button
						onclick={copyCommand}
						class="flex items-center gap-1 text-xs text-primary-600 dark:text-primary-400 hover:text-primary-700 dark:hover:text-primary-300 transition-colors"
					>
						{#if copied}
							<Check class="w-3 h-3" />
							{i18n.t('settingsComp.copied')}
						{:else}
							<Copy class="w-3 h-3" />
							{i18n.t('settingsComp.copy')}
						{/if}
					</button>
				</div>
				<code class="text-sm font-mono text-gray-800 dark:text-gray-200 break-all">{builtCommand}</code>
			</div>
		{/if}
	</div>

	<!-- Scheduling Reference -->
	<div class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-5">
		<div class="flex items-center gap-2 mb-1">
			<Clock class="w-4 h-4 text-gray-500 dark:text-gray-400" />
			<h3 class="text-base font-semibold text-gray-900 dark:text-white">{i18n.t('settingsComp.schedulingTitle')}</h3>
		</div>
		<p class="text-xs text-gray-500 dark:text-gray-400 mb-4">
			{i18n.t('settingsComp.schedulingDesc')}
		</p>

		<div class="space-y-3">
			<div class="bg-gray-50 dark:bg-gray-900/50 rounded-lg p-3">
				<code class="text-sm font-semibold text-gray-800 dark:text-gray-200">/loop</code>
				<span class="text-xs text-gray-500 dark:text-gray-400 ml-2">{i18n.t('settingsComp.intervalCommand')}</span>
				<p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
					{i18n.t('settingsComp.loopCommandDesc')}
				</p>
				<p class="text-xs text-gray-600 dark:text-gray-300 mt-1 font-mono">
					{i18n.t('settingsComp.exampleLabel')} <code>/loop 5m /commit</code>
				</p>
			</div>

			<div class="bg-gray-50 dark:bg-gray-900/50 rounded-lg p-3">
				<code class="text-sm font-semibold text-gray-800 dark:text-gray-200">/schedule</code>
				<p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
					{i18n.t('settingsComp.scheduleCommandDesc')}
				</p>
				<p class="text-xs text-gray-600 dark:text-gray-300 mt-1 font-mono">
					{i18n.t('settingsComp.exampleLabel')} <code>/schedule "daily at 9am" review open PRs</code>
				</p>
			</div>

			<div class="bg-gray-50 dark:bg-gray-900/50 rounded-lg p-3">
				<p class="text-xs font-medium text-gray-600 dark:text-gray-300 mb-1">{i18n.t('settingsComp.availableTools')}</p>
				<div class="flex flex-wrap gap-2">
					{#each ['CronCreate', 'CronList', 'CronDelete'] as tool}
						<code class="text-xs px-2 py-0.5 rounded bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300">{tool}</code>
					{/each}
				</div>
				<p class="text-xs text-gray-500 dark:text-gray-400 mt-1.5">
					{i18n.t('settingsComp.cronToolsDesc')}
				</p>
			</div>
		</div>
	</div>
</div>
