<script lang="ts">
	import type { StatusLineSegment, SegmentType } from '$lib/types';
	import { SEGMENT_TYPES, SEGMENT_COLORS } from '$lib/types';
	import { i18n } from '$lib/i18n';
	import { GripVertical, X, Eye, EyeOff, CornerDownLeft } from 'lucide-svelte';

	type Props = {
		segment: StatusLineSegment;
		isSelected?: boolean;
		onSelect?: (seg: StatusLineSegment) => void;
		onRemove?: (seg: StatusLineSegment) => void;
		onToggle?: (seg: StatusLineSegment) => void;
	};

	let { segment, isSelected = false, onSelect, onRemove, onToggle }: Props = $props();

	const segmentLabels = $derived<Partial<Record<SegmentType, string>>>({
		model: i18n.t('statuslineComp.segModel'),
		cost: i18n.t('statuslineComp.segCost'),
		context: i18n.t('statuslineComp.segContext'),
		context_remaining: i18n.t('statuslineComp.segContextRemaining'),
		cwd: i18n.t('statuslineComp.segCwd'),
		project_dir: i18n.t('statuslineComp.segProjectDir'),
		tokens_in: i18n.t('statuslineComp.segTokensIn'),
		tokens_out: i18n.t('statuslineComp.segTokensOut'),
		duration: i18n.t('statuslineComp.segDuration'),
		api_duration: i18n.t('statuslineComp.segApiDuration'),
		lines_changed: i18n.t('statuslineComp.segLinesChanged'),
		git_branch: i18n.t('statuslineComp.segGitBranch'),
		git_status: i18n.t('statuslineComp.segGitStatus'),
		session_id: i18n.t('statuslineComp.segSessionId'),
		version: i18n.t('statuslineComp.segVersion'),
		agent_name: i18n.t('statuslineComp.segAgentName'),
		five_hour_usage: i18n.t('statuslineComp.segFiveHourUsage'),
		weekly_usage: i18n.t('statuslineComp.segWeeklyUsage'),
		vim_mode: i18n.t('statuslineComp.segVimMode'),
		separator: i18n.t('statuslineComp.separator'),
		line_break: i18n.t('statuslineComp.lineBreak'),
		custom_text: i18n.t('statuslineComp.segCustomText')
	});

	const meta = $derived(SEGMENT_TYPES.find((t) => t.type === segment.type));
	const segmentLabel = $derived(segmentLabels[segment.type] || meta?.label || segment.type);
	const colorHex = $derived(SEGMENT_COLORS.find((c) => c.value === segment.color)?.hex || '#f8fafc');
	const bgColorHex = $derived(segment.bgColor ? SEGMENT_COLORS.find((c) => c.value === segment.bgColor)?.hex : undefined);
	const isLineBreak = $derived(segment.type === 'line_break');
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
{#if isLineBreak}
	<div
		onclick={() => onSelect?.(segment)}
		class="flex items-center gap-2 w-full px-3 py-1 rounded-lg border border-dashed transition-all text-xs cursor-pointer select-none
			{isSelected
				? 'border-primary-400 dark:border-primary-500 bg-primary-50/50 dark:bg-primary-900/20'
				: 'border-gray-300 dark:border-gray-600 bg-gray-50 dark:bg-gray-800/50 hover:border-gray-400 dark:hover:border-gray-500'}"
	>
		<GripVertical class="w-3 h-3 text-gray-400 shrink-0 cursor-grab" />
		<CornerDownLeft class="w-3 h-3 text-gray-400 shrink-0" />
		<span class="text-gray-400 dark:text-gray-500 font-medium">{i18n.t('statuslineComp.newLine')}</span>
		<div class="flex-1 border-t border-dashed border-gray-300 dark:border-gray-600 mx-2"></div>
		<button
			onclick={(e) => { e.stopPropagation(); onRemove?.(segment); }}
			class="p-0.5 rounded text-gray-400 hover:text-red-500 shrink-0"
			title={i18n.t('common.remove')}
		>
			<X class="w-3 h-3" />
		</button>
	</div>
{:else}
	<div
		onclick={() => onSelect?.(segment)}
		class="flex items-center gap-2 px-3 py-2 rounded-lg border text-left transition-all text-sm cursor-pointer select-none
			{isSelected
				? 'border-primary-400 dark:border-primary-500 bg-primary-50 dark:bg-primary-900/30 ring-1 ring-primary-200 dark:ring-primary-700'
				: 'border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 hover:border-gray-300 dark:hover:border-gray-600'}
			{!segment.enabled ? 'opacity-50' : ''}"
	>
		<GripVertical class="w-3.5 h-3.5 text-gray-400 shrink-0 cursor-grab" />
		<span
			class="w-3 h-3 rounded-full shrink-0 flex items-center justify-center"
			style="{bgColorHex ? `background-color: ${bgColorHex}` : 'background-color: transparent'}"
		>
			<span class="w-1.5 h-1.5 rounded-full" style="background-color: {colorHex}"></span>
		</span>
		<span class="truncate font-medium text-gray-700 dark:text-gray-300">
			{segmentLabel}
		</span>
		<div class="ml-auto flex items-center gap-1 shrink-0">
			<button
				onclick={(e) => { e.stopPropagation(); onToggle?.(segment); }}
				class="p-1 rounded text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
				title={segment.enabled ? i18n.t('common.disable') : i18n.t('common.enable')}
			>
				{#if segment.enabled}
					<Eye class="w-3.5 h-3.5" />
				{:else}
					<EyeOff class="w-3.5 h-3.5" />
				{/if}
			</button>
			<button
				onclick={(e) => { e.stopPropagation(); onRemove?.(segment); }}
				class="p-1 rounded text-gray-400 hover:text-red-500"
				title={i18n.t('common.remove')}
			>
				<X class="w-3.5 h-3.5" />
			</button>
		</div>
	</div>
{/if}
