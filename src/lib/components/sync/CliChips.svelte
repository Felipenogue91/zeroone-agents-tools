<script lang="ts">
	import CliChip from './CliChip.svelte';
	import type { Cli, TargetState } from '$lib/types/sync';
	import { CLIS } from '$lib/types/sync';

	type Props = {
		targets: Partial<Record<Cli, TargetState>>;
		disabled?: boolean;
		onToggle?: (cli: Cli, enabled: boolean) => void;
	};

	let { targets, disabled = false, onToggle }: Props = $props();
</script>

<div class="flex items-center gap-1" role="group">
	{#each CLIS as cli (cli)}
		<CliChip
			{cli}
			state={targets[cli]}
			{disabled}
			onToggle={onToggle ? (enabled) => onToggle?.(cli, enabled) : undefined}
		/>
	{/each}
</div>
