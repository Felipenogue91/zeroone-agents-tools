<script lang="ts">
	type Props = { rows?: number };
	let { rows = 5 }: Props = $props();
</script>

<div class="space-y-2" aria-busy="true" aria-live="polite">
	{#each Array(rows) as _, i (i)}
		<div class="flex items-center gap-3 px-4 py-2">
			<div class="skeleton h-4 w-4"></div>
			<div class="skeleton h-4 flex-1 max-w-[220px]"></div>
			<div class="skeleton h-4 w-40 ml-auto"></div>
		</div>
	{/each}
</div>
