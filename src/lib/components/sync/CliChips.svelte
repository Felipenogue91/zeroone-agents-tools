<script lang="ts">
	import CliChip from './CliChip.svelte';
	import type { Cli, TargetState } from '$lib/types/sync';
	import { CLIS } from '$lib/types/sync';

	type Props = {
		targets: Partial<Record<Cli, TargetState>>;
		disabled?: boolean;
		onToggle?: (cli: Cli, enabled: boolean) => void;
		/** CLIs shown but not switchable here, with the reason. */
		locked?: Partial<Record<Cli, string>>;
	};

	let { targets, disabled = false, onToggle, locked = {} }: Props = $props();
</script>

<div class="flex items-center gap-1" role="group">
	{#each CLIS as cli (cli)}
		<CliChip
			{cli}
			state={targets[cli]}
			{disabled}
			lockedReason={locked[cli]}
			onToggle={onToggle ? (enabled) => onToggle?.(cli, enabled) : undefined}
		/>
	{/each}
</div>
