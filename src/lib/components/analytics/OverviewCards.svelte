<script lang="ts">
	import { Hash, MessageSquare, Wrench, Calendar, Clock, DollarSign } from 'lucide-svelte';
	import type { LongestSession } from '$lib/types';
	import { formatCompactNumber, formatDuration, formatCost } from '$lib/types/usage';
	import { i18n } from '$lib/i18n';

	type Props = {
		totalSessions: number | null;
		totalMessages: number | null;
		totalToolCalls: number;
		firstSessionDate: string | null;
		longestSession: LongestSession | null;
		lastComputedDate: string | null;
		totalCostUSD?: number;
	};

	let {
		totalSessions,
		totalMessages,
		totalToolCalls,
		firstSessionDate,
		longestSession,
		lastComputedDate,
		totalCostUSD
	}: Props = $props();

	function formatDate(iso: string | null): string {
		if (!iso) return i18n.t('analyticsComp.na');
		try {
			return new Date(iso).toLocaleDateString(undefined, {
				year: 'numeric',
				month: 'short',
				day: 'numeric'
			});
		} catch {
			return iso;
		}
	}

	const cards = $derived([
		{
			label: i18n.t('analyticsComp.totalSessions'),
			value: totalSessions != null ? formatCompactNumber(totalSessions) : '0',
			icon: Hash,
			color: 'bg-blue-500'
		},
		{
			label: i18n.t('analyticsComp.totalMessages'),
			value: totalMessages != null ? formatCompactNumber(totalMessages) : '0',
			icon: MessageSquare,
			color: 'bg-purple-500'
		},
		{
			label: i18n.t('analyticsComp.toolCalls'),
			value: formatCompactNumber(totalToolCalls),
			icon: Wrench,
			color: 'bg-amber-500'
		},
		{
			label: i18n.t('analyticsComp.firstSession'),
			value: formatDate(firstSessionDate),
			icon: Calendar,
			color: 'bg-green-500'
		},
		{
			label: i18n.t('analyticsComp.longestSession'),
			value: longestSession ? formatDuration(longestSession.duration) : i18n.t('analyticsComp.na'),
			subtitle: longestSession ? i18n.t('analyticsComp.messagesCount', { count: longestSession.messageCount }) : undefined,
			icon: Clock,
			color: 'bg-rose-500'
		},
		...(totalCostUSD != null
			? [
					{
						label: i18n.t('analyticsComp.estApiCost'),
						value: formatCost(totalCostUSD),
						subtitle: i18n.t('analyticsComp.ifBilledAtApiRates'),
						icon: DollarSign,
						color: 'bg-emerald-500'
					}
				]
			: [])
	]);
</script>

<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-6 gap-3">
	{#each cards as card}
		<div
			class="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-4"
		>
			<div class="flex items-start justify-between">
				<div>
					<p class="text-sm font-medium text-gray-500 dark:text-gray-400">{card.label}</p>
					<p class="text-2xl font-bold text-gray-900 dark:text-white mt-0.5">{card.value}</p>
					{#if card.subtitle}
						<p class="text-xs text-gray-400 dark:text-gray-500 mt-0.5">{card.subtitle}</p>
					{/if}
				</div>
				<div class="{card.color} p-2 rounded-lg">
					<card.icon class="w-4 h-4 text-white" />
				</div>
			</div>
		</div>
	{/each}
</div>

{#if lastComputedDate}
	<p class="text-xs text-gray-400 dark:text-gray-500 mt-2">
		{i18n.t('analyticsComp.lastUpdated', { date: formatDate(lastComputedDate) })}
	</p>
{/if}
