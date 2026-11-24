<script lang="ts">
	import type { SegmentType, StatusLineSegment } from '$lib/types';
	import { SEGMENT_TYPES } from '$lib/types';
	import { i18n } from '$lib/i18n';
	import { Plus, GitBranch, BarChart3, Monitor, SeparatorVertical, WrapText } from 'lucide-svelte';

	type Props = {
		onAdd: (segment: StatusLineSegment) => void;
	};

	let { onAdd }: Props = $props();

	let openGroup = $state<string | null>(null);

	interface SegmentGroup {
		key: string;
		label: string;
		icon: typeof Plus;
		types: SegmentType[];
	}

	const groups: SegmentGroup[] = $derived([
		{
			key: 'model',
			label: i18n.t('statuslineComp.groupModel'),
			icon: Monitor,
			types: ['model', 'version', 'session_id', 'agent_name', 'vim_mode']
		},
		{
			key: 'usage',
			label: i18n.t('statuslineComp.groupUsage'),
			icon: BarChart3,
			types: ['cost', 'context', 'context_remaining', 'tokens_in', 'tokens_out', 'five_hour_usage', 'weekly_usage', 'duration', 'api_duration', 'lines_changed']
		},
		{
			key: 'git',
			label: i18n.t('statuslineComp.groupGit'),
			icon: GitBranch,
			types: ['git_branch', 'git_status', 'cwd', 'project_dir']
		},
		{
			key: 'other',
			label: i18n.t('statuslineComp.groupCustom'),
			icon: Plus,
			types: ['custom_text']
		}
	]);

	const segmentInfo = $derived<Partial<Record<SegmentType, { label: string; description: string }>>>({
		model: { label: i18n.t('statuslineComp.segModel'), description: i18n.t('statuslineComp.segDescModel') },
		cost: { label: i18n.t('statuslineComp.segCost'), description: i18n.t('statuslineComp.segDescCost') },
		context: { label: i18n.t('statuslineComp.segContext'), description: i18n.t('statuslineComp.segDescContext') },
		context_remaining: { label: i18n.t('statuslineComp.segContextRemaining'), description: i18n.t('statuslineComp.segDescContextRemaining') },
		cwd: { label: i18n.t('statuslineComp.segCwd'), description: i18n.t('statuslineComp.segDescCwd') },
		project_dir: { label: i18n.t('statuslineComp.segProjectDir'), description: i18n.t('statuslineComp.segDescProjectDir') },
		tokens_in: { label: i18n.t('statuslineComp.segTokensIn'), description: i18n.t('statuslineComp.segDescTokensIn') },
		tokens_out: { label: i18n.t('statuslineComp.segTokensOut'), description: i18n.t('statuslineComp.segDescTokensOut') },
		duration: { label: i18n.t('statuslineComp.segDuration'), description: i18n.t('statuslineComp.segDescDuration') },
		api_duration: { label: i18n.t('statuslineComp.segApiDuration'), description: i18n.t('statuslineComp.segDescApiDuration') },
		lines_changed: { label: i18n.t('statuslineComp.segLinesChanged'), description: i18n.t('statuslineComp.segDescLinesChanged') },
		git_branch: { label: i18n.t('statuslineComp.segGitBranch'), description: i18n.t('statuslineComp.segDescGitBranch') },
		git_status: { label: i18n.t('statuslineComp.segGitStatus'), description: i18n.t('statuslineComp.segDescGitStatus') },
		session_id: { label: i18n.t('statuslineComp.segSessionId'), description: i18n.t('statuslineComp.segDescSessionId') },
		version: { label: i18n.t('statuslineComp.segVersion'), description: i18n.t('statuslineComp.segDescVersion') },
		agent_name: { label: i18n.t('statuslineComp.segAgentName'), description: i18n.t('statuslineComp.segDescAgentName') },
		five_hour_usage: { label: i18n.t('statuslineComp.segFiveHourUsage'), description: i18n.t('statuslineComp.segDescFiveHourUsage') },
		weekly_usage: { label: i18n.t('statuslineComp.segWeeklyUsage'), description: i18n.t('statuslineComp.segDescWeeklyUsage') },
		vim_mode: { label: i18n.t('statuslineComp.segVimMode'), description: i18n.t('statuslineComp.segDescVimMode') },
		custom_text: { label: i18n.t('statuslineComp.segCustomText'), description: i18n.t('statuslineComp.segDescCustomText') }
	});

	function addSegment(type: SegmentType) {
		const meta = SEGMENT_TYPES.find((t) => t.type === type);
		const segment: StatusLineSegment = {
			id: crypto.randomUUID(),
			type,
			enabled: true,
			color: meta?.defaultColor || 'white',
			format: meta?.formats?.[0]?.value,
			position: Date.now(),
			separatorChar: type === 'separator' ? '|' : undefined,
			customText: type === 'custom_text' ? '' : undefined
		};
		onAdd(segment);
		openGroup = null;
	}

	function addSeparator() {
		addSegment('separator');
	}

	function toggleGroup(key: string) {
		openGroup = openGroup === key ? null : key;
	}

	function getSegmentMeta(type: SegmentType) {
		return SEGMENT_TYPES.find((t) => t.type === type);
	}
</script>

<div class="flex items-center gap-1.5">
	{#each groups as group}
		<div class="relative">
			<button
				onclick={() => toggleGroup(group.key)}
				class="btn btn-secondary text-xs px-2.5 py-1.5 {openGroup === group.key ? 'ring-2 ring-primary-400' : ''}"
			>
				<svelte:component this={group.icon} class="w-3.5 h-3.5 mr-1" />
				{group.label}
			</button>

			{#if openGroup === group.key}
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div class="fixed inset-0 z-40" onclick={() => (openGroup = null)}></div>
				<div class="absolute left-0 mt-2 w-64 bg-white dark:bg-gray-800 rounded-lg shadow-lg border border-gray-200 dark:border-gray-700 py-1 z-50 max-h-72 overflow-auto">
					{#each group.types as type}
						{@const meta = getSegmentMeta(type)}
						{@const info = segmentInfo[type]}
						{#if meta}
							<button
								onclick={() => addSegment(type)}
								class="w-full flex flex-col px-3 py-2 text-left hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
							>
								<span class="text-sm font-medium text-gray-700 dark:text-gray-300">{info?.label || meta.label}</span>
								<span class="text-xs text-gray-400 dark:text-gray-500">{info?.description || meta.description}</span>
							</button>
						{/if}
					{/each}
				</div>
			{/if}
		</div>
	{/each}

	<button
		onclick={addSeparator}
		class="btn btn-secondary text-xs px-2.5 py-1.5"
		title={i18n.t('statuslineComp.addSeparatorTitle')}
	>
		<SeparatorVertical class="w-3.5 h-3.5 mr-1" />
		{i18n.t('statuslineComp.separator')}
	</button>

	<button
		onclick={() => addSegment('line_break')}
		class="btn btn-secondary text-xs px-2.5 py-1.5"
		title={i18n.t('statuslineComp.addLineBreakTitle')}
	>
		<WrapText class="w-3.5 h-3.5 mr-1" />
		{i18n.t('statuslineComp.lineBreak')}
	</button>
</div>
