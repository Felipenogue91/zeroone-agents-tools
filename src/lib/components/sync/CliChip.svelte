<script lang="ts">
	import { Asterisk, SquareTerminal, Orbit, Check, Minus, Plug, Link2, Ban } from 'lucide-svelte';
	import type { Cli, TargetState } from '$lib/types/sync';
	import { CLI_LABEL, CLI_SHORT, isToggleable } from '$lib/types/sync';
	import { i18n } from '$lib/i18n';

	type Props = {
		cli: Cli;
		state?: TargetState;
		disabled?: boolean;
		/** Called with the new value; omit for a read-only chip. */
		onToggle?: (enabled: boolean) => void;
		/** Plain CLI label (no on/off meaning). */
		label?: boolean;
	};

	let { cli, state, disabled = false, onToggle, label = false }: Props = $props();

	const cliIcon = { claude: Asterisk, codex: SquareTerminal, agy: Orbit };
	let Icon = $derived(cliIcon[cli]);
	let kind = $derived(state?.state ?? 'off');
	let toggleable = $derived(!!onToggle && isToggleable(state) && !disabled);

	let tooltip = $derived.by(() => {
		const name = CLI_LABEL[cli];
		switch (kind) {
			case 'on':
				return i18n.t('sync.chip.on', { cli: name });
			case 'off':
				return i18n.t('sync.chip.off', { cli: name });
			case 'providedByPlugin':
				return i18n.t('sync.chip.plugin', { cli: name });
			case 'native':
				return i18n.t('sync.chip.native', { cli: name });
			case 'unsupported':
				return state && 'reason' in state ? state.reason : i18n.t('sync.chip.unsupported', { cli: name });
			default:
				return name;
		}
	});

	const styles: Record<string, string> = {
		on: 'border-primary-300 bg-primary-50 text-primary-700 dark:border-primary-700 dark:bg-primary-900/40 dark:text-primary-200',
		native: 'border-primary-200 bg-primary-50/60 text-primary-700 dark:border-primary-800 dark:bg-primary-950/60 dark:text-primary-300',
		providedByPlugin: 'border-dashed border-gray-300 text-gray-600 dark:border-gray-600 dark:text-gray-300',
		off: 'border-gray-200 text-gray-500 line-through decoration-gray-400/60 dark:border-gray-700 dark:text-gray-400',
		unsupported: 'border-gray-200 text-gray-400 opacity-70 dark:border-gray-700 dark:text-gray-500'
	};
</script>

{#if label}
	<span class="chip border-gray-200 text-gray-700 dark:border-gray-600 dark:text-gray-200" title={CLI_LABEL[cli]}>
		<Icon class="w-3 h-3" aria-hidden="true" />
		<span>{CLI_SHORT[cli]}</span>
	</span>
{:else if onToggle}
	<button
		type="button"
		class="chip {styles[kind]} {toggleable ? 'hover:border-primary-400 cursor-pointer' : 'cursor-default'}"
		aria-pressed={kind === 'on'}
		aria-label="{CLI_LABEL[cli]}: {tooltip}"
		title={tooltip}
		disabled={!toggleable}
		onclick={() => toggleable && onToggle?.(kind !== 'on')}
	>
		<Icon class="w-3 h-3" aria-hidden="true" />
		<span>{CLI_SHORT[cli]}</span>
		{@render stateIcon()}
	</button>
{:else}
	<span class="chip {styles[kind]}" title={tooltip} aria-label="{CLI_LABEL[cli]}: {tooltip}">
		<Icon class="w-3 h-3" aria-hidden="true" />
		<span>{CLI_SHORT[cli]}</span>
		{@render stateIcon()}
	</span>
{/if}

{#snippet stateIcon()}
	{#if kind === 'on'}
		<Check class="w-3 h-3" aria-hidden="true" />
	{:else if kind === 'native'}
		<Link2 class="w-3 h-3" aria-hidden="true" />
	{:else if kind === 'providedByPlugin'}
		<Plug class="w-3 h-3" aria-hidden="true" />
	{:else if kind === 'unsupported'}
		<Ban class="w-3 h-3" aria-hidden="true" />
	{:else}
		<Minus class="w-3 h-3" aria-hidden="true" />
	{/if}
{/snippet}
