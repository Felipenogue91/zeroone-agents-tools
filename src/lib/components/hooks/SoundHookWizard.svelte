<script lang="ts">
	import { hookLibrary, soundLibrary, notifications } from '$lib/stores';
	import { i18n } from '$lib/i18n';
	import { SOUND_HOOK_PRESETS, getDefaultSound, type HookEventType, type SoundMethod } from '$lib/types';
	import { SoundPicker } from '$lib/components/sounds';
	import {
		X,
		ChevronRight,
		ChevronLeft,
		CheckCircle,
		Bell,
		ShieldAlert,
		BellRing,
		Volume2,
		Terminal,
		FileCode,
		Check,
		Loader2
	} from 'lucide-svelte';

	type Props = {
		onClose: () => void;
		onComplete?: () => void;
	};

	let { onClose, onComplete }: Props = $props();

	// Wizard state
	let step = $state(1);
	let selectedPresetId = $state<string | null>(null);
	let selectedEvents = $state<HookEventType[]>([]);
	let selectedSound = $state(getDefaultSound());
	let selectedMethod = $state<SoundMethod>('shell');
	let isCreating = $state(false);

	// Get the icon for a preset
	function getPresetIcon(id: string) {
		switch (id) {
			case 'task-complete':
				return CheckCircle;
			case 'permission-required':
				return ShieldAlert;
			case 'full-suite':
				return BellRing;
			default:
				return Bell;
		}
	}

	function selectPreset(id: string) {
		selectedPresetId = id;
		const preset = SOUND_HOOK_PRESETS.find((p) => p.id === id);
		if (preset) {
			selectedEvents = [...preset.events];
		}
	}

	function toggleEvent(event: HookEventType) {
		if (selectedEvents.includes(event)) {
			selectedEvents = selectedEvents.filter((e) => e !== event);
		} else {
			selectedEvents = [...selectedEvents, event];
		}
		// Clear preset selection if events don't match any preset
		const matchingPreset = SOUND_HOOK_PRESETS.find(
			(p) => p.events.length === selectedEvents.length && p.events.every((e) => selectedEvents.includes(e))
		);
		selectedPresetId = matchingPreset?.id || null;
	}

	function canProceed(): boolean {
		switch (step) {
			case 1:
				return selectedEvents.length > 0;
			case 2:
				return !!selectedSound;
			case 3:
				return true;
			default:
				return false;
		}
	}

	async function handleCreate() {
		if (selectedEvents.length === 0 || !selectedSound) return;

		isCreating = true;
		try {
			// Deploy Python script if using Python method
			if (selectedMethod === 'python') {
				await soundLibrary.deployNotificationScript();
			}

			// Create the hooks
			const hooks = await hookLibrary.createSoundNotificationHooks(
				selectedEvents,
				selectedSound,
				selectedMethod
			);

			notifications.success(i18n.t('hookModal.createdSoundHooks', { count: hooks.length }));
			onComplete?.();
			onClose();
		} catch (e) {
			notifications.error(i18n.t('hookModal.createHooksFailed'));
			console.error(e);
		} finally {
			isCreating = false;
		}
	}

	const allEvents = $derived<{ event: HookEventType; label: string; description: string }[]>([
		{ event: 'Stop', label: i18n.t('hookModal.eventTaskComplete'), description: i18n.t('hookModal.eventTaskCompleteDesc') },
		{ event: 'SubagentStop', label: i18n.t('hookModal.eventSubagentComplete'), description: i18n.t('hookModal.eventSubagentCompleteDesc') },
		{ event: 'Notification', label: i18n.t('hookModal.eventNotification'), description: i18n.t('hookModal.eventNotificationDesc') },
		{ event: 'SessionStart', label: i18n.t('hookModal.eventSessionStart'), description: i18n.t('hookModal.eventSessionStartDesc') },
		{ event: 'SessionEnd', label: i18n.t('hookModal.eventSessionEnd'), description: i18n.t('hookModal.eventSessionEndDesc') }
	]);
</script>

<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
	<div class="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-2xl w-full max-h-[90vh] flex flex-col overflow-hidden">
		<!-- Header -->
		<div class="flex-shrink-0 flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-gray-700">
			<div class="flex items-center gap-3">
				<div class="w-10 h-10 rounded-lg bg-orange-100 dark:bg-orange-900/50 flex items-center justify-center">
					<Volume2 class="w-5 h-5 text-orange-600 dark:text-orange-400" />
				</div>
				<div>
					<h2 class="text-lg font-semibold text-gray-900 dark:text-white">{i18n.t('hookModal.soundWizardTitle')}</h2>
					<p class="text-sm text-gray-500">{i18n.t('hookModal.stepOfTotal', { step, total: 3 })}</p>
				</div>
			</div>
			<button onclick={onClose} class="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300">
				<X class="w-5 h-5" />
			</button>
		</div>

		<!-- Progress bar -->
		<div class="flex-shrink-0 h-1 bg-gray-200 dark:bg-gray-700">
			<div
				class="h-full bg-orange-500 transition-all duration-300"
				style="width: {(step / 3) * 100}%"
			></div>
		</div>

		<!-- Content -->
		<div class="flex-1 overflow-y-auto p-6">
			{#if step === 1}
				<!-- Step 1: Choose events -->
				<div class="space-y-6">
					<div>
						<h3 class="text-lg font-medium text-gray-900 dark:text-white mb-2">{i18n.t('hookModal.chooseEvents')}</h3>
						<p class="text-sm text-gray-500">{i18n.t('hookModal.chooseEventsDesc')}</p>
					</div>

					<!-- Presets -->
					<div>
						<p class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">{i18n.t('hookModal.quickPresets')}</p>
						<div class="grid grid-cols-3 gap-3">
							{#each SOUND_HOOK_PRESETS as preset (preset.id)}
								{@const Icon = getPresetIcon(preset.id)}
								<button
									onclick={() => selectPreset(preset.id)}
									class="flex flex-col items-center p-4 rounded-lg border-2 transition-all
										{selectedPresetId === preset.id
										? 'border-orange-500 bg-orange-50 dark:bg-orange-900/20'
										: 'border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600'}"
								>
									<Icon class="w-8 h-8 mb-2 {selectedPresetId === preset.id ? 'text-orange-600' : 'text-gray-400'}" />
									<span class="text-sm font-medium text-gray-900 dark:text-white">{preset.name}</span>
									<span class="text-xs text-gray-500 text-center mt-1">{preset.description}</span>
								</button>
							{/each}
						</div>
					</div>

					<!-- Individual events -->
					<div>
						<p class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">{i18n.t('hookModal.orSelectIndividualEvents')}</p>
						<div class="space-y-2">
							{#each allEvents as { event, label, description } (event)}
								<button
									onclick={() => toggleEvent(event)}
									class="w-full flex items-center gap-3 p-3 rounded-lg border transition-all text-left
										{selectedEvents.includes(event)
										? 'border-orange-500 bg-orange-50 dark:bg-orange-900/20'
										: 'border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600'}"
								>
									<div class="w-5 h-5 rounded border flex items-center justify-center
										{selectedEvents.includes(event)
										? 'bg-orange-500 border-orange-500'
										: 'border-gray-300 dark:border-gray-600'}">
										{#if selectedEvents.includes(event)}
											<Check class="w-3 h-3 text-white" />
										{/if}
									</div>
									<div>
										<p class="font-medium text-gray-900 dark:text-white">{label}</p>
										<p class="text-xs text-gray-500">{description}</p>
									</div>
								</button>
							{/each}
						</div>
					</div>
				</div>
			{:else if step === 2}
				<!-- Step 2: Choose sound -->
				<div class="space-y-6">
					<div>
						<h3 class="text-lg font-medium text-gray-900 dark:text-white mb-2">{i18n.t('hookModal.chooseSound')}</h3>
						<p class="text-sm text-gray-500">{i18n.t('hookModal.chooseSoundDesc')}</p>
					</div>

					<div>
						<SoundPicker
							value={selectedSound}
							onchange={(path) => (selectedSound = path)}
						/>
					</div>

					{#if selectedSound}
						<div class="p-4 bg-gray-50 dark:bg-gray-800/50 rounded-lg">
							<p class="text-sm text-gray-600 dark:text-gray-400">
								<span class="font-medium">{i18n.t('hookModal.selectedLabel')}</span>
								<span class="font-mono text-xs ml-2">{selectedSound}</span>
							</p>
						</div>
					{/if}
				</div>
			{:else if step === 3}
				<!-- Step 3: Choose method and review -->
				<div class="space-y-6">
					<div>
						<h3 class="text-lg font-medium text-gray-900 dark:text-white mb-2">{i18n.t('hookModal.playbackMethod')}</h3>
						<p class="text-sm text-gray-500">{i18n.t('hookModal.playbackMethodDesc')}</p>
					</div>

					<div class="grid grid-cols-2 gap-4">
						<button
							onclick={() => (selectedMethod = 'shell')}
							class="flex flex-col items-center p-4 rounded-lg border-2 transition-all
								{selectedMethod === 'shell'
								? 'border-gray-500 bg-gray-50 dark:bg-gray-700'
								: 'border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600'}"
						>
							<Terminal class="w-8 h-8 mb-2 {selectedMethod === 'shell' ? 'text-gray-700 dark:text-gray-300' : 'text-gray-400'}" />
							<span class="font-medium text-gray-900 dark:text-white">{i18n.t('hookModal.shellCommand')}</span>
							<span class="text-xs text-gray-500 text-center mt-1">{i18n.t('hookModal.shellCommandDesc')}</span>
						</button>

						<button
							onclick={() => (selectedMethod = 'python')}
							class="flex flex-col items-center p-4 rounded-lg border-2 transition-all
								{selectedMethod === 'python'
								? 'border-violet-500 bg-violet-50 dark:bg-violet-900/20'
								: 'border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600'}"
						>
							<FileCode class="w-8 h-8 mb-2 {selectedMethod === 'python' ? 'text-violet-600' : 'text-gray-400'}" />
							<span class="font-medium text-gray-900 dark:text-white">{i18n.t('hookModal.pythonScript')}</span>
							<span class="text-xs text-gray-500 text-center mt-1">{i18n.t('hookModal.pythonScriptDesc')}</span>
						</button>
					</div>

					<!-- Summary -->
					<div class="p-4 bg-gray-50 dark:bg-gray-800/50 rounded-lg space-y-3">
						<p class="text-sm font-medium text-gray-700 dark:text-gray-300">{i18n.t('hookModal.summary')}</p>
						<div class="space-y-2 text-sm">
							<div class="flex justify-between">
								<span class="text-gray-500">{i18n.t('hookModal.eventsLabel')}</span>
								<span class="text-gray-900 dark:text-white">{selectedEvents.join(', ')}</span>
							</div>
							<div class="flex justify-between">
								<span class="text-gray-500">{i18n.t('hookModal.soundLabel')}</span>
								<span class="text-gray-900 dark:text-white font-mono text-xs">
									{selectedSound.split('/').pop()}
								</span>
							</div>
							<div class="flex justify-between">
								<span class="text-gray-500">{i18n.t('hookModal.methodLabel')}</span>
								<span class="text-gray-900 dark:text-white">
									{selectedMethod === 'shell' ? i18n.t('hookModal.shellCommand') : i18n.t('hookModal.pythonScript')}
								</span>
							</div>
						</div>
					</div>

					<p class="text-xs text-gray-500">
						{i18n.t(selectedEvents.length === 1 ? 'hookModal.willCreateOne' : 'hookModal.willCreatePlural', { count: selectedEvents.length })}
					</p>
				</div>
			{/if}
		</div>

		<!-- Footer -->
		<div class="flex-shrink-0 flex items-center justify-between px-6 py-4 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50">
			<button
				onclick={() => (step = Math.max(1, step - 1))}
				class="btn btn-secondary"
				disabled={step === 1}
			>
				<ChevronLeft class="w-4 h-4 mr-1" />
				{i18n.t('hookModal.back')}
			</button>

			{#if step < 3}
				<button
					onclick={() => (step = step + 1)}
					class="btn btn-primary"
					disabled={!canProceed()}
				>
					{i18n.t('hookModal.next')}
					<ChevronRight class="w-4 h-4 ml-1" />
				</button>
			{:else}
				<button
					onclick={handleCreate}
					class="btn btn-primary"
					disabled={isCreating || !canProceed()}
				>
					{#if isCreating}
						<Loader2 class="w-4 h-4 mr-2 animate-spin" />
						{i18n.t('hookModal.creating')}
					{:else}
						<Check class="w-4 h-4 mr-2" />
						{i18n.t('hookModal.createHooks')}
					{/if}
				</button>
			{/if}
		</div>
	</div>
</div>
