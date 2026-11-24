<script lang="ts">
	import type { ClaudeSettings, AgentTeamMember } from '$lib/types';
	import { TEAMMATE_MODES, AGENT_TEAM_PERMISSION_MODES, AVAILABLE_MODEL_SHORTCUTS } from '$lib/types';
	import { i18n } from '$lib/i18n';
	import { Save, Plus, X, Users, ChevronDown, ChevronUp } from 'lucide-svelte';

	type Props = {
		settings: ClaudeSettings;
		onsave: (settings: ClaudeSettings) => void;
	};

	let { settings, onsave }: Props = $props();

	let enabled = $state(settings.agentTeam?.enabled ?? false);
	let teammateMode = $state(settings.teammateMode ?? '');
	let members = $state<AgentTeamMember[]>(settings.agentTeam?.members ?? []);
	let expandedIndex = $state<number | null>(null);

	// New member form
	let newName = $state('');
	let newModel = $state('');
	let newDescription = $state('');
	let newPermissionMode = $state('');

	$effect(() => {
		enabled = settings.agentTeam?.enabled ?? false;
		teammateMode = settings.teammateMode ?? '';
		members = settings.agentTeam?.members ?? [];
		expandedIndex = null;
	});

	function addMember() {
		const trimmedName = newName.trim();
		if (!trimmedName) return;
		if (members.some((m) => m.name === trimmedName)) return;

		const member: AgentTeamMember = { name: trimmedName };
		if (newModel) member.model = newModel;
		if (newDescription.trim()) member.description = newDescription.trim();
		if (newPermissionMode) member.permissionMode = newPermissionMode;

		members = [...members, member];
		newName = '';
		newModel = '';
		newDescription = '';
		newPermissionMode = '';
	}

	function removeMember(index: number) {
		members = members.filter((_, i) => i !== index);
		if (expandedIndex === index) expandedIndex = null;
		else if (expandedIndex !== null && expandedIndex > index) expandedIndex--;
	}

	function updateMember(index: number, field: keyof AgentTeamMember, value: string) {
		const updated = [...members];
		const member = { ...updated[index] };
		if (value) {
			member[field] = value;
		} else if (field !== 'name') {
			delete member[field];
		}
		updated[index] = member;
		members = updated;
	}

	function toggleExpand(index: number) {
		expandedIndex = expandedIndex === index ? null : index;
	}

	function handleSave() {
		const agentTeam =
			enabled || members.length > 0
				? {
						enabled: enabled || undefined,
						members: members.length > 0 ? members : undefined
					}
				: undefined;

		onsave({
			...settings,
			teammateMode: teammateMode || undefined,
			agentTeam
		});
	}

	const isDuplicateName = $derived(
		newName.trim() !== '' && members.some((m) => m.name === newName.trim())
	);
</script>

<div class="space-y-6">
	<!-- Enable toggle + teammate mode -->
	<div
		class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-5"
	>
		<div class="flex items-center gap-3 mb-1">
			<Users class="w-5 h-5 text-primary-500" />
			<h3 class="text-base font-semibold text-gray-900 dark:text-white">{i18n.t('settingsComp.agentTeamsTitle')}</h3>
		</div>
		<p class="text-xs text-gray-500 dark:text-gray-400 mb-4">
			{i18n.t('settingsComp.agentTeamsDesc')}
		</p>

		<div class="space-y-4">
			<div class="flex items-center justify-between">
				<div>
					<label for="agent-team-enabled" class="text-sm font-medium text-gray-700 dark:text-gray-300">
						{i18n.t('settingsComp.enableAgentTeams')}
					</label>
					<p class="text-xs text-gray-500 dark:text-gray-400">
						{i18n.t('settingsComp.enableAgentTeamsDesc')}
					</p>
				</div>
				<button
					id="agent-team-enabled"
					role="switch"
					aria-checked={enabled}
					aria-label={i18n.t('settingsComp.enableAgentTeams')}
					onclick={() => (enabled = !enabled)}
					class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors
						{enabled ? 'bg-primary-500' : 'bg-gray-300 dark:bg-gray-600'}"
				>
					<span
						class="inline-block h-4 w-4 rounded-full bg-white transition-transform
							{enabled ? 'translate-x-6' : 'translate-x-1'}"
					></span>
				</button>
			</div>

			<div>
				<label for="teammate-mode" class="text-sm font-medium text-gray-700 dark:text-gray-300">
					{i18n.t('settingsComp.teammateDisplayMode')}
				</label>
				<p class="text-xs text-gray-500 dark:text-gray-400 mb-2">
					{i18n.t('settingsComp.teammateDisplayModeDesc')}
				</p>
				<select id="teammate-mode" bind:value={teammateMode} class="input w-full">
					{#each TEAMMATE_MODES as mode}
						<option value={mode.value}>{mode.label}</option>
					{/each}
				</select>
			</div>
		</div>
	</div>

	<!-- Team members list -->
	<div
		class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-5"
	>
		<h3 class="text-base font-semibold text-gray-900 dark:text-white mb-1">{i18n.t('settingsComp.teamMembersTitle')}</h3>
		<p class="text-xs text-gray-500 dark:text-gray-400 mb-4">
			{i18n.t('settingsComp.teamMembersDesc')}
		</p>

		{#if members.length > 0}
			<div class="space-y-2 mb-4">
				{#each members as member, i}
					<div
						class="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden"
					>
						<!-- Member header row -->
						<div class="flex items-center gap-3 px-3 py-2.5 bg-gray-50 dark:bg-gray-750">
							<button
								onclick={() => toggleExpand(i)}
								class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
								aria-label={expandedIndex === i ? i18n.t('settingsComp.collapse') : i18n.t('settingsComp.expand')}
							>
								{#if expandedIndex === i}
									<ChevronUp class="w-4 h-4" />
								{:else}
									<ChevronDown class="w-4 h-4" />
								{/if}
							</button>
							<span class="text-sm font-medium text-gray-900 dark:text-white flex-1">
								{member.name}
							</span>
							{#if member.model}
								<span
									class="text-xs px-2 py-0.5 rounded-full bg-primary-100 dark:bg-primary-900/40 text-primary-700 dark:text-primary-300"
								>
									{member.model}
								</span>
							{/if}
							{#if member.permissionMode}
								<span
									class="text-xs px-2 py-0.5 rounded-full bg-amber-100 dark:bg-amber-900/40 text-amber-700 dark:text-amber-300"
								>
									{member.permissionMode}
								</span>
							{/if}
							<button
								onclick={() => removeMember(i)}
								class="text-red-400 hover:text-red-600 dark:hover:text-red-400"
								aria-label={i18n.t('settingsComp.removeItem', { name: member.name })}
							>
								<X class="w-4 h-4" />
							</button>
						</div>

						<!-- Expanded detail editor -->
						{#if expandedIndex === i}
							<div class="px-3 py-3 space-y-3 border-t border-gray-200 dark:border-gray-700">
								<div>
									<label
										for="member-name-{i}"
										class="text-xs font-medium text-gray-600 dark:text-gray-400"
									>
										{i18n.t('common.name')}
									</label>
									<input
										id="member-name-{i}"
										type="text"
										value={member.name}
										oninput={(e) =>
											updateMember(i, 'name', (e.target as HTMLInputElement).value)}
										class="input text-sm w-full mt-1"
									/>
								</div>
								<div>
									<label
										for="member-model-{i}"
										class="text-xs font-medium text-gray-600 dark:text-gray-400"
									>
										{i18n.t('subagentForm.model')}
									</label>
									<select
										id="member-model-{i}"
										value={member.model ?? ''}
										onchange={(e) =>
											updateMember(
												i,
												'model',
												(e.target as HTMLSelectElement).value
											)}
										class="input text-sm w-full mt-1"
									>
										<option value="">{i18n.t('settingsComp.defaultInheritFromLead')}</option>
										{#each AVAILABLE_MODEL_SHORTCUTS as model}
											<option value={model.value}>{model.label}</option>
										{/each}
									</select>
								</div>
								<div>
									<label
										for="member-permission-{i}"
										class="text-xs font-medium text-gray-600 dark:text-gray-400"
									>
										{i18n.t('subagentForm.permissionMode')}
									</label>
									<select
										id="member-permission-{i}"
										value={member.permissionMode ?? ''}
										onchange={(e) =>
											updateMember(
												i,
												'permissionMode',
												(e.target as HTMLSelectElement).value
											)}
										class="input text-sm w-full mt-1"
									>
										{#each AGENT_TEAM_PERMISSION_MODES as mode}
											<option value={mode.value}>{mode.label}</option>
										{/each}
									</select>
								</div>
								<div>
									<label
										for="member-desc-{i}"
										class="text-xs font-medium text-gray-600 dark:text-gray-400"
									>
										{i18n.t('settingsComp.descOrRole')}
									</label>
									<input
										id="member-desc-{i}"
										type="text"
										value={member.description ?? ''}
										oninput={(e) =>
											updateMember(
												i,
												'description',
												(e.target as HTMLInputElement).value
											)}
										placeholder={i18n.t('settingsComp.memberDescPlaceholder')}
										class="input text-sm w-full mt-1"
									/>
								</div>
							</div>
						{/if}
					</div>
				{/each}
			</div>
		{:else}
			<p class="text-xs text-gray-500 dark:text-gray-400 italic mb-4">
				{i18n.t('settingsComp.noTeamMembers')}
			</p>
		{/if}

		<!-- Add new member -->
		<div class="border border-dashed border-gray-300 dark:border-gray-600 rounded-lg p-3 space-y-3">
			<p class="text-xs font-medium text-gray-500 dark:text-gray-400">{i18n.t('settingsComp.addTeamMember')}</p>
			<div class="flex items-center gap-2">
				<input
					type="text"
					bind:value={newName}
					placeholder={i18n.t('settingsComp.memberNamePlaceholder')}
					class="input text-sm flex-1"
					class:border-red-400={isDuplicateName}
					onkeydown={(e) => e.key === 'Enter' && addMember()}
				/>
				<select bind:value={newModel} class="input text-sm w-28">
					<option value="">{i18n.t('subagentForm.model')}</option>
					{#each AVAILABLE_MODEL_SHORTCUTS as model}
						<option value={model.value}>{model.label}</option>
					{/each}
				</select>
				<button
					onclick={addMember}
					disabled={!newName.trim() || isDuplicateName}
					class="btn btn-ghost"
					aria-label={i18n.t('settingsComp.addMemberAria')}
				>
					<Plus class="w-4 h-4" />
				</button>
			</div>
			{#if isDuplicateName}
				<p class="text-xs text-red-500">{i18n.t('settingsComp.duplicateMemberName')}</p>
			{/if}
			<div class="flex gap-2">
				<select bind:value={newPermissionMode} class="input text-sm flex-1">
					{#each AGENT_TEAM_PERMISSION_MODES as mode}
						<option value={mode.value}>{mode.label}</option>
					{/each}
				</select>
				<input
					type="text"
					bind:value={newDescription}
					placeholder={i18n.t('settingsComp.memberRolePlaceholder')}
					class="input text-sm flex-1"
					onkeydown={(e) => e.key === 'Enter' && addMember()}
				/>
			</div>
		</div>
	</div>

	<div class="flex justify-end">
		<button onclick={handleSave} class="btn btn-primary">
			<Save class="w-4 h-4 mr-2" />
			{i18n.t('settingsComp.saveAgentTeams')}
		</button>
	</div>
</div>
