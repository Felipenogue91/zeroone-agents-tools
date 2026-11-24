<script lang="ts">
	import type { Project, Mcp, Skill, SubAgent, Command, Hook, ProjectSkill, ProjectSubAgent, ProjectCommand, ProjectHook } from '$lib/types';
	import { mcpLibrary, projectsStore, notifications, skillLibrary, subagentLibrary, commandLibrary, hookLibrary } from '$lib/stores';
	import { i18n } from '$lib/i18n';
	import { Plus, Minus, Plug, Globe, Server, Radio, Sparkles, Bot, Terminal, Search, Zap, SquareCheck, Square, Power, PowerOff, Trash2 } from 'lucide-svelte';

	type Props = {
		project: Project;
	};

	let { project: projectProp }: Props = $props();

	// Tab state
	type Tab = 'mcps' | 'skills' | 'agents' | 'commands' | 'hooks';
	let activeTab = $state<Tab>('mcps');

	// Search state
	let searchQuery = $state('');

	// Multi-select state
	let selectedIds = $state<Set<number>>(new Set());
	let bulkLoading = $state(false);

	// Helpers for selection
	function toggleSelection(id: number) {
		const next = new Set(selectedIds);
		if (next.has(id)) next.delete(id);
		else next.add(id);
		selectedIds = next;
	}

	function toggleSelectAll(assignmentIds: number[]) {
		const allSelected = assignmentIds.every((id) => selectedIds.has(id));
		if (allSelected) {
			selectedIds = new Set();
		} else {
			selectedIds = new Set(assignmentIds);
		}
	}

	function clearSelection() {
		selectedIds = new Set();
	}

	// Bulk action handlers
	async function bulkToggle(enabled: boolean) {
		if (selectedIds.size === 0) return;
		bulkLoading = true;
		try {
			const ids = [...selectedIds];
			if (activeTab === 'mcps') {
				for (const id of ids) await projectsStore.toggleProjectMcp(id, enabled);
			} else if (activeTab === 'skills') {
				for (const id of ids) await skillLibrary.toggleProjectSkill(id, enabled);
			} else if (activeTab === 'agents') {
				for (const id of ids) await subagentLibrary.toggleProjectSubAgent(id, enabled);
			} else if (activeTab === 'commands') {
				for (const id of ids) await commandLibrary.toggleProjectCommand(id, enabled);
			} else if (activeTab === 'hooks') {
				for (const id of ids) await hookLibrary.toggleProjectHook(id, enabled);
			}
			await projectsStore.syncProjectConfig(project.id);
			if (activeTab !== 'mcps') await loadProjectData();
			notifications.success(
				enabled
					? (ids.length === 1 ? i18n.t('projComp.enabledOne') : i18n.t('projComp.enabledMany', { count: ids.length }))
					: (ids.length === 1 ? i18n.t('projComp.disabledOne') : i18n.t('projComp.disabledMany', { count: ids.length }))
			);
			clearSelection();
		} catch (err) {
			notifications.error(i18n.t(enabled ? 'projComp.enableItemsFailed' : 'projComp.disableItemsFailed'));
			console.error(err);
		} finally {
			bulkLoading = false;
		}
	}

	async function bulkRemove() {
		if (selectedIds.size === 0) return;
		bulkLoading = true;
		try {
			const ids = [...selectedIds];
			if (activeTab === 'mcps') {
				for (const id of ids) {
					const assignment = project.assignedMcps.find((a) => a.id === id);
					if (assignment) await projectsStore.removeMcpFromProject(project.id, assignment.mcpId);
				}
			} else if (activeTab === 'skills') {
				for (const id of ids) {
					const assignment = projectSkills.find((a) => a.id === id);
					if (assignment) await skillLibrary.removeFromProject(project.id, assignment.skillId);
				}
			} else if (activeTab === 'agents') {
				for (const id of ids) {
					const assignment = projectSubAgents.find((a) => a.id === id);
					if (assignment) await subagentLibrary.removeFromProject(project.id, assignment.subagentId);
				}
			} else if (activeTab === 'commands') {
				for (const id of ids) {
					const assignment = projectCommands.find((a) => a.id === id);
					if (assignment) await commandLibrary.removeFromProject(project.id, assignment.commandId);
				}
			} else if (activeTab === 'hooks') {
				for (const id of ids) {
					const assignment = projectHooks.find((a) => a.id === id);
					if (assignment) await hookLibrary.removeFromProject(project.id, assignment.hookId);
				}
			}
			await projectsStore.syncProjectConfig(project.id);
			if (activeTab !== 'mcps') await loadProjectData();
			notifications.success(
				ids.length === 1 ? i18n.t('projComp.removedOne') : i18n.t('projComp.removedMany', { count: ids.length })
			);
			clearSelection();
		} catch (err) {
			notifications.error(i18n.t('projComp.removeItemsFailed'));
			console.error(err);
		} finally {
			bulkLoading = false;
		}
	}


	const typeIcons = {
		stdio: Plug,
		sse: Globe,
		http: Server,
		ws: Radio
	};

	const typeColors = {
		stdio: 'bg-purple-100 text-purple-600 dark:bg-purple-900/50 dark:text-purple-400',
		sse: 'bg-green-100 text-green-600 dark:bg-green-900/50 dark:text-green-400',
		http: 'bg-blue-100 text-blue-600 dark:bg-blue-900/50 dark:text-blue-400',
		ws: 'bg-orange-100 text-orange-600 dark:bg-orange-900/50 dark:text-orange-400'
	};

	// Get current project from store (updates after loadProjects)
	let project = $derived(
		projectsStore.getProjectById(projectProp.id) ?? projectProp
	);

	// MCP state
	let assignedMcpIds = $derived(project.assignedMcps.map((a) => a.mcpId));
	let enabledGlobalMcpIds = $derived(
		(projectsStore.globalMcps ?? []).filter((g) => g.isEnabled).map((g) => g.mcpId)
	);
	let inheritedGlobalMcps = $derived(
		(projectsStore.globalMcps ?? []).filter((g) => g.isEnabled && !assignedMcpIds.includes(g.mcpId))
	);
	let availableMcps = $derived(
		mcpLibrary.mcps.filter((mcp) => !assignedMcpIds.includes(mcp.id) && !enabledGlobalMcpIds.includes(mcp.id))
	);
	let filteredAvailableMcps = $derived(
		searchQuery.trim()
			? availableMcps.filter((mcp) => {
					const query = searchQuery.toLowerCase();
					return mcp.name.toLowerCase().includes(query) || mcp.description?.toLowerCase().includes(query);
				})
			: availableMcps
	);

	// Skills state
	let projectSkills = $state<ProjectSkill[]>([]);
	let assignedSkillIds = $derived(projectSkills.map((ps) => ps.skillId));
	let enabledGlobalSkillIds = $derived(
		(skillLibrary.globalSkills ?? []).filter((g) => g.isEnabled).map((g) => g.skillId)
	);
	let inheritedGlobalSkills = $derived(
		(skillLibrary.globalSkills ?? []).filter((g) => g.isEnabled && !assignedSkillIds.includes(g.skillId))
	);
	let availableSkills = $derived(
		skillLibrary.skills.filter((skill) => !assignedSkillIds.includes(skill.id) && !enabledGlobalSkillIds.includes(skill.id))
	);
	let filteredAvailableSkills = $derived(
		searchQuery.trim()
			? availableSkills.filter((skill) => {
					const query = searchQuery.toLowerCase();
					return skill.name.toLowerCase().includes(query) || skill.description?.toLowerCase().includes(query);
				})
			: availableSkills
	);

	// SubAgents state
	let projectSubAgents = $state<ProjectSubAgent[]>([]);
	let assignedSubAgentIds = $derived(projectSubAgents.map((pa) => pa.subagentId));
	let enabledGlobalSubAgentIds = $derived(
		(subagentLibrary.globalSubAgents ?? []).filter((g) => g.isEnabled).map((g) => g.subagentId)
	);
	let inheritedGlobalSubAgents = $derived(
		(subagentLibrary.globalSubAgents ?? []).filter((g) => g.isEnabled && !assignedSubAgentIds.includes(g.subagentId))
	);
	let availableSubAgents = $derived(
		subagentLibrary.subagents.filter((agent) => !assignedSubAgentIds.includes(agent.id) && !enabledGlobalSubAgentIds.includes(agent.id))
	);
	let filteredAvailableSubAgents = $derived(
		searchQuery.trim()
			? availableSubAgents.filter((agent) => {
					const query = searchQuery.toLowerCase();
					return agent.name.toLowerCase().includes(query) || agent.description?.toLowerCase().includes(query);
				})
			: availableSubAgents
	);

	// Commands state
	let projectCommands = $state<ProjectCommand[]>([]);
	let assignedCommandIds = $derived(projectCommands.map((pc) => pc.commandId));
	let enabledGlobalCommandIds = $derived(
		(commandLibrary.globalCommands ?? []).filter((g) => g.isEnabled).map((g) => g.commandId)
	);
	let inheritedGlobalCommands = $derived(
		(commandLibrary.globalCommands ?? []).filter((g) => g.isEnabled && !assignedCommandIds.includes(g.commandId))
	);
	let availableCommands = $derived(
		commandLibrary.commands.filter((cmd) => !assignedCommandIds.includes(cmd.id) && !enabledGlobalCommandIds.includes(cmd.id))
	);
	let filteredAvailableCommands = $derived(
		searchQuery.trim()
			? availableCommands.filter((cmd) => {
					const query = searchQuery.toLowerCase();
					return cmd.name.toLowerCase().includes(query) || cmd.description?.toLowerCase().includes(query);
				})
			: availableCommands
	);

	// Hooks state
	let projectHooks = $state<ProjectHook[]>([]);
	let assignedHookIds = $derived(projectHooks.map((ph) => ph.hookId));
	let enabledGlobalHookIds = $derived(
		(hookLibrary.globalHooks ?? []).filter((g) => g.isEnabled).map((g) => g.hookId)
	);
	let inheritedGlobalHooks = $derived(
		(hookLibrary.globalHooks ?? []).filter((g) => g.isEnabled && !assignedHookIds.includes(g.hookId))
	);
	let availableHooks = $derived(
		hookLibrary.hooks.filter((hook) => !assignedHookIds.includes(hook.id) && !enabledGlobalHookIds.includes(hook.id))
	);
	let filteredAvailableHooks = $derived(
		searchQuery.trim()
			? availableHooks.filter((hook) => {
					const query = searchQuery.toLowerCase();
					return hook.name.toLowerCase().includes(query) || hook.description?.toLowerCase().includes(query);
				})
			: availableHooks
	);

	// Derived selection helpers (must be after all state declarations)
	let currentAssignmentIds = $derived<number[]>(
		activeTab === 'mcps' ? project.assignedMcps.map((a) => a.id) :
		activeTab === 'skills' ? projectSkills.map((a) => a.id) :
		activeTab === 'agents' ? projectSubAgents.map((a) => a.id) :
		activeTab === 'commands' ? projectCommands.map((a) => a.id) :
		projectHooks.map((a) => a.id)
	);

	let allSelected = $derived(currentAssignmentIds.length > 0 && currentAssignmentIds.every((id) => selectedIds.has(id)));
	let selectedCount = $derived([...selectedIds].filter((id) => currentAssignmentIds.includes(id)).length);

	// Load project data
	$effect(() => {
		loadProjectData();
	});

	async function loadProjectData() {
		try {
			projectSkills = await skillLibrary.getProjectSkills(project.id);
			projectSubAgents = await subagentLibrary.getProjectSubAgents(project.id);
			projectCommands = await commandLibrary.getProjectCommands(project.id);
			projectHooks = await hookLibrary.getProjectHooks(project.id);
		} catch (err) {
			console.error('Failed to load project data:', err);
		}
	}

	// MCP handlers
	async function handleAddMcp(mcp: Mcp) {
		try {
			await projectsStore.assignMcpToProject(project.id, mcp.id);
			await projectsStore.syncProjectConfig(project.id);
			notifications.success(i18n.t('projComp.addedItem', { name: mcp.name, project: project.name }));
		} catch (err) {
			notifications.error(i18n.t('global.addMcpFailed'));
			console.error(err);
		}
	}

	async function handleRemoveMcp(mcpId: number) {
		try {
			const mcp = mcpLibrary.getMcpById(mcpId);
			await projectsStore.removeMcpFromProject(project.id, mcpId);
			await projectsStore.syncProjectConfig(project.id);
			notifications.success(i18n.t('projComp.removedItem', { name: mcp?.name || 'MCP', project: project.name }));
		} catch (err) {
			notifications.error(i18n.t('global.removeMcpFailed'));
			console.error(err);
		}
	}

	async function handleToggleMcp(assignmentId: number, enabled: boolean) {
		try {
			await projectsStore.toggleProjectMcp(assignmentId, enabled);
			await projectsStore.syncProjectConfig(project.id);
		} catch (err) {
			notifications.error(i18n.t('global.toggleMcpFailed'));
			console.error(err);
		}
	}

	// Skill handlers
	async function handleAddSkill(skill: Skill) {
		try {
			await skillLibrary.assignToProject(project.id, skill.id);
			await projectsStore.syncProjectConfig(project.id);
			await loadProjectData();
			notifications.success(i18n.t('projComp.addedItem', { name: skill.name, project: project.name }));
		} catch (err) {
			notifications.error(i18n.t('global.addSkillFailed'));
			console.error(err);
		}
	}

	async function handleRemoveSkill(skillId: number) {
		try {
			const skill = skillLibrary.getSkillById(skillId);
			await skillLibrary.removeFromProject(project.id, skillId);
			await projectsStore.syncProjectConfig(project.id);
			await loadProjectData();
			notifications.success(i18n.t('projComp.removedItem', { name: skill?.name || 'Skill', project: project.name }));
		} catch (err) {
			notifications.error(i18n.t('global.removeSkillFailed'));
			console.error(err);
		}
	}

	async function handleToggleSkill(assignmentId: number, enabled: boolean) {
		try {
			await skillLibrary.toggleProjectSkill(assignmentId, enabled);
			await projectsStore.syncProjectConfig(project.id);
			await loadProjectData();
		} catch (err) {
			notifications.error(i18n.t('global.toggleSkillFailed'));
			console.error(err);
		}
	}

	// SubAgent handlers
	async function handleAddSubAgent(agent: SubAgent) {
		try {
			await subagentLibrary.assignToProject(project.id, agent.id);
			await projectsStore.syncProjectConfig(project.id);
			await loadProjectData();
			notifications.success(i18n.t('projComp.addedItem', { name: agent.name, project: project.name }));
		} catch (err) {
			notifications.error(i18n.t('global.addAgentFailed'));
			console.error(err);
		}
	}

	async function handleRemoveSubAgent(agentId: number) {
		try {
			const agent = subagentLibrary.getSubAgentById(agentId);
			await subagentLibrary.removeFromProject(project.id, agentId);
			await projectsStore.syncProjectConfig(project.id);
			await loadProjectData();
			notifications.success(i18n.t('projComp.removedItem', { name: agent?.name || i18n.t('projComp.agent'), project: project.name }));
		} catch (err) {
			notifications.error(i18n.t('global.removeAgentFailed'));
			console.error(err);
		}
	}

	async function handleToggleSubAgent(assignmentId: number, enabled: boolean) {
		try {
			await subagentLibrary.toggleProjectSubAgent(assignmentId, enabled);
			await projectsStore.syncProjectConfig(project.id);
			await loadProjectData();
		} catch (err) {
			notifications.error(i18n.t('global.toggleAgentFailed'));
			console.error(err);
		}
	}

	// Command handlers
	async function handleAddCommand(command: Command) {
		try {
			await commandLibrary.assignToProject(project.id, command.id);
			await projectsStore.syncProjectConfig(project.id);
			await loadProjectData();
			notifications.success(i18n.t('projComp.addedItem', { name: command.name, project: project.name }));
		} catch (err) {
			notifications.error(i18n.t('global.addCommandFailed'));
			console.error(err);
		}
	}

	async function handleRemoveCommand(commandId: number) {
		try {
			const command = commandLibrary.getCommandById(commandId);
			await commandLibrary.removeFromProject(project.id, commandId);
			await projectsStore.syncProjectConfig(project.id);
			await loadProjectData();
			notifications.success(i18n.t('projComp.removedItem', { name: command?.name || i18n.t('hook.command'), project: project.name }));
		} catch (err) {
			notifications.error(i18n.t('global.removeCommandFailed'));
			console.error(err);
		}
	}

	async function handleToggleCommand(assignmentId: number, enabled: boolean) {
		try {
			await commandLibrary.toggleProjectCommand(assignmentId, enabled);
			await projectsStore.syncProjectConfig(project.id);
			await loadProjectData();
		} catch (err) {
			notifications.error(i18n.t('global.toggleCommandFailed'));
			console.error(err);
		}
	}

	// Hook handlers
	async function handleAddHook(hook: Hook) {
		try {
			await hookLibrary.assignToProject(project.id, hook.id);
			await projectsStore.syncProjectConfig(project.id);
			await loadProjectData();
			notifications.success(i18n.t('projComp.addedItem', { name: hook.name, project: project.name }));
		} catch (err) {
			notifications.error(i18n.t('projComp.addHookFailed'));
			console.error(err);
		}
	}

	async function handleRemoveHook(hookId: number) {
		try {
			const hook = hookLibrary.getHookById(hookId);
			await hookLibrary.removeFromProject(project.id, hookId);
			await projectsStore.syncProjectConfig(project.id);
			await loadProjectData();
			notifications.success(i18n.t('projComp.removedItem', { name: hook?.name || i18n.t('projComp.hook'), project: project.name }));
		} catch (err) {
			notifications.error(i18n.t('projComp.removeHookFailed'));
			console.error(err);
		}
	}

	async function handleToggleHook(assignmentId: number, enabled: boolean) {
		try {
			await hookLibrary.toggleProjectHook(assignmentId, enabled);
			await projectsStore.syncProjectConfig(project.id);
			await loadProjectData();
		} catch (err) {
			notifications.error(i18n.t('projComp.toggleHookFailed'));
			console.error(err);
		}
	}

	// Clear search and selection when switching tabs
	function handleTabChange(tab: Tab) {
		searchQuery = '';
		clearSelection();
		activeTab = tab;
	}

	function getSearchPlaceholder(): string {
		switch (activeTab) {
			case 'mcps': return i18n.t('projComp.searchAvailableMcps');
			case 'skills': return i18n.t('projComp.searchAvailableSkills');
			case 'agents': return i18n.t('projComp.searchAvailableAgents');
			case 'commands': return i18n.t('projComp.searchAvailableCommands');
			case 'hooks': return i18n.t('projComp.searchAvailableHooks');
		}
	}

	// Helper to get event type label
	function getEventLabel(eventType: string): string {
		return eventType.replace(/([A-Z])/g, ' $1').trim();
	}
</script>

<!-- Sub-tabs -->
<div class="flex border-b border-gray-200 dark:border-gray-700 px-4 mt-1">
	<button
		onclick={() => handleTabChange('mcps')}
		class="flex items-center gap-2 px-4 py-3 text-sm font-medium border-b-2 transition-colors {activeTab === 'mcps' ? 'border-primary-500 text-primary-600 dark:text-primary-400' : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200'}"
	>
		<Plug class="w-4 h-4" />
		{i18n.t('global.tabMcps')} ({project.assignedMcps.length}{inheritedGlobalMcps.length > 0 ? ` + ${inheritedGlobalMcps.length} ${i18n.t('dashboard.global')}` : ''})
	</button>
	<button
		onclick={() => handleTabChange('skills')}
		class="flex items-center gap-2 px-4 py-3 text-sm font-medium border-b-2 transition-colors {activeTab === 'skills' ? 'border-primary-500 text-primary-600 dark:text-primary-400' : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200'}"
	>
		<Sparkles class="w-4 h-4" />
		{i18n.t('global.tabSkills')} ({projectSkills.length}{inheritedGlobalSkills.length > 0 ? ` + ${inheritedGlobalSkills.length} ${i18n.t('dashboard.global')}` : ''})
	</button>
	<button
		onclick={() => handleTabChange('agents')}
		class="flex items-center gap-2 px-4 py-3 text-sm font-medium border-b-2 transition-colors {activeTab === 'agents' ? 'border-primary-500 text-primary-600 dark:text-primary-400' : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200'}"
	>
		<Bot class="w-4 h-4" />
		{i18n.t('global.tabAgents')} ({projectSubAgents.length}{inheritedGlobalSubAgents.length > 0 ? ` + ${inheritedGlobalSubAgents.length} ${i18n.t('dashboard.global')}` : ''})
	</button>
	<button
		onclick={() => handleTabChange('commands')}
		class="flex items-center gap-2 px-4 py-3 text-sm font-medium border-b-2 transition-colors {activeTab === 'commands' ? 'border-primary-500 text-primary-600 dark:text-primary-400' : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200'}"
	>
		<Terminal class="w-4 h-4" />
		{i18n.t('global.tabCommands')} ({projectCommands.length}{inheritedGlobalCommands.length > 0 ? ` + ${inheritedGlobalCommands.length} ${i18n.t('dashboard.global')}` : ''})
	</button>
	<button
		onclick={() => handleTabChange('hooks')}
		class="flex items-center gap-2 px-4 py-3 text-sm font-medium border-b-2 transition-colors {activeTab === 'hooks' ? 'border-primary-500 text-primary-600 dark:text-primary-400' : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200'}"
	>
		<Zap class="w-4 h-4" />
		{i18n.t('dashboard.hooks')} ({projectHooks.length}{inheritedGlobalHooks.length > 0 ? ` + ${inheritedGlobalHooks.length} ${i18n.t('dashboard.global')}` : ''})
	</button>
</div>

<!-- Bulk Action Bar -->
{#if selectedCount > 0}
	<div class="sticky top-0 z-10 mx-6 mt-4 flex items-center justify-between gap-3 px-4 py-2.5 bg-primary-50 dark:bg-primary-900/30 border border-primary-200 dark:border-primary-800 rounded-xl">
		<span class="text-sm font-medium text-primary-700 dark:text-primary-300">
			{i18n.t('projComp.selectedCount', { count: selectedCount })}
		</span>
		<div class="flex items-center gap-2">
			<button
				onclick={() => bulkToggle(true)}
				disabled={bulkLoading}
				class="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg bg-green-100 text-green-700 hover:bg-green-200 dark:bg-green-900/40 dark:text-green-400 dark:hover:bg-green-900/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-500 transition-colors disabled:opacity-50"
			>
				<Power class="w-3.5 h-3.5" />
				{i18n.t('common.enable')}
			</button>
			<button
				onclick={() => bulkToggle(false)}
				disabled={bulkLoading}
				class="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg bg-gray-100 text-gray-700 hover:bg-gray-200 dark:bg-gray-700 dark:text-gray-300 dark:hover:bg-gray-600 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-500 transition-colors disabled:opacity-50"
			>
				<PowerOff class="w-3.5 h-3.5" />
				{i18n.t('common.disable')}
			</button>
			<button
				onclick={bulkRemove}
				disabled={bulkLoading}
				class="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg bg-red-100 text-red-700 hover:bg-red-200 dark:bg-red-900/40 dark:text-red-400 dark:hover:bg-red-900/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-500 transition-colors disabled:opacity-50"
			>
				<Trash2 class="w-3.5 h-3.5" />
				{i18n.t('common.remove')}
			</button>
			<button
				onclick={clearSelection}
				class="ml-1 px-2 py-1.5 text-xs text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-500 rounded transition-colors"
			>
				{i18n.t('common.cancel')}
			</button>
		</div>
	</div>
{/if}

<!-- Content -->
<div class="flex-1 overflow-auto p-6 space-y-6">
	{#if activeTab === 'mcps'}
		{#if inheritedGlobalMcps.length > 0}
			<div>
				<div class="flex items-center justify-between mb-3">
					<h3 class="text-sm font-medium text-gray-700 dark:text-gray-300 flex items-center gap-1.5">
						<Globe class="w-4 h-4 text-primary-500" />
						{i18n.t('projComp.globalInherited', { count: inheritedGlobalMcps.length })}
					</h3>
					<span class="text-xs text-gray-400 dark:text-gray-500">{i18n.t('projComp.activeViaGlobal')}</span>
				</div>
				<div class="space-y-2">
					{#each inheritedGlobalMcps as globalItem (globalItem.id)}
						{@const mcp = mcpLibrary.getMcpById(globalItem.mcpId) ?? globalItem.mcp}
						<div class="flex items-center justify-between p-3 bg-primary-50/50 dark:bg-primary-900/10 border border-primary-100 dark:border-primary-800/40 rounded-lg">
							<div class="flex items-center gap-3">
								<div class="w-8 h-8 rounded-lg {typeColors[mcp.type]} flex items-center justify-center">
									<svelte:component this={typeIcons[mcp.type]} class="w-4 h-4" />
								</div>
								<div>
									<span class="font-medium text-gray-900 dark:text-white">{mcp.name}</span>
									<span class="text-xs text-gray-500 dark:text-gray-400 ml-2">({mcp.type})</span>
								</div>
							</div>
							<span class="px-2 py-0.5 text-xs font-medium rounded-full bg-primary-100 text-primary-700 dark:bg-primary-900/40 dark:text-primary-300">
								{i18n.t('projComp.globalBadge')}
							</span>
						</div>
					{/each}
				</div>
			</div>
		{/if}

		<!-- Assigned MCPs -->
		<div>
			<div class="flex items-center gap-3 mb-3">
				{#if project.assignedMcps.length > 1}
					<button
						onclick={() => toggleSelectAll(project.assignedMcps.map(a => a.id))}
						class="p-0.5 text-gray-400 hover:text-primary-500 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:ring-offset-1 rounded transition-colors"
						title={i18n.t(allSelected ? 'projComp.deselectAll' : 'projComp.selectAll')}
					>
						{#if allSelected}
							<SquareCheck class="w-4 h-4 text-primary-500" />
						{:else}
							<Square class="w-4 h-4" />
						{/if}
					</button>
				{/if}
				<h3 class="text-sm font-medium text-gray-700 dark:text-gray-300">
					{i18n.t('projComp.assignedMcps', { count: project.assignedMcps.length })}
				</h3>
			</div>
			{#if project.assignedMcps.length > 0}
				<div class="space-y-2">
					{#each project.assignedMcps as assignment (assignment.id)}
						{@const mcp = mcpLibrary.getMcpById(assignment.mcpId) ?? assignment.mcp}
						<div class="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg {selectedIds.has(assignment.id) ? 'ring-2 ring-primary-500/50' : ''}">
							<div class="flex items-center gap-3">
								<button
									onclick={() => toggleSelection(assignment.id)}
									class="p-0.5 text-gray-400 hover:text-primary-500 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:ring-offset-1 rounded transition-colors flex-shrink-0"
								>
									{#if selectedIds.has(assignment.id)}
										<SquareCheck class="w-4 h-4 text-primary-500" />
									{:else}
										<Square class="w-4 h-4" />
									{/if}
								</button>
								<div class="w-8 h-8 rounded-lg {typeColors[mcp.type]} flex items-center justify-center">
									<svelte:component this={typeIcons[mcp.type]} class="w-4 h-4" />
								</div>
								<div>
									<span class="font-medium text-gray-900 dark:text-white {!assignment.isEnabled ? 'line-through opacity-50' : ''}">
										{mcp.name}
									</span>
									<span class="text-xs text-gray-500 dark:text-gray-400 ml-2">({mcp.type})</span>
								</div>
							</div>
							<div class="flex items-center gap-3">
								<button
									onclick={() => handleToggleMcp(assignment.id, !assignment.isEnabled)}
									class="relative inline-flex h-5 w-9 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none {assignment.isEnabled ? 'bg-primary-600' : 'bg-gray-300 dark:bg-gray-600'}"
									role="switch"
									aria-checked={assignment.isEnabled}
									title={i18n.t(assignment.isEnabled ? 'common.disable' : 'common.enable')}
								>
									<span class="pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out {assignment.isEnabled ? 'translate-x-4' : 'translate-x-0'}"></span>
								</button>
								<button
									onclick={() => handleRemoveMcp(assignment.mcpId)}
									class="p-1.5 text-gray-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg transition-colors"
									title={i18n.t('projComp.removeFromProject')}
								>
									<Minus class="w-4 h-4" />
								</button>
							</div>
						</div>
					{/each}
				</div>
			{:else}
				<div class="text-center py-6 bg-gray-50 dark:bg-gray-700/30 rounded-lg">
					<p class="text-gray-500 dark:text-gray-400">{i18n.t('projComp.noMcpsAssigned')}</p>
					<p class="text-sm text-gray-400 dark:text-gray-500 mt-1">{i18n.t('projComp.addMcpsHint')}</p>
				</div>
			{/if}
		</div>

		<!-- Available MCPs -->
		<div>
			<div class="flex items-center justify-between mb-3">
				<h3 class="text-sm font-medium text-gray-700 dark:text-gray-300">
					{i18n.t('projComp.availableMcps', { count: availableMcps.length })}
				</h3>
				{#if availableMcps.length > 3}
					<div class="relative w-48">
						<Search class="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-gray-400" />
						<input
							type="text"
							bind:value={searchQuery}
							placeholder={getSearchPlaceholder()}
							class="input pl-8 py-1.5 text-sm"
						/>
					</div>
				{/if}
			</div>
			{#if availableMcps.length > 0}
				{#if filteredAvailableMcps.length > 0}
					<div class="space-y-2">
						{#each filteredAvailableMcps as mcp (mcp.id)}
							<div class="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
								<div class="flex items-center gap-3">
									<div class="w-8 h-8 rounded-lg {typeColors[mcp.type]} flex items-center justify-center">
										<svelte:component this={typeIcons[mcp.type]} class="w-4 h-4" />
									</div>
									<div>
										<span class="font-medium text-gray-900 dark:text-white">{mcp.name}</span>
										<span class="text-xs text-gray-500 dark:text-gray-400 ml-2">({mcp.type})</span>
									</div>
								</div>
								<button
									onclick={() => handleAddMcp(mcp)}
									class="p-1.5 text-gray-400 hover:text-green-500 hover:bg-green-50 dark:hover:bg-green-900/20 rounded-lg transition-colors"
									title={i18n.t('projComp.addToProject')}
								>
									<Plus class="w-4 h-4" />
								</button>
							</div>
						{/each}
					</div>
				{:else}
					<div class="text-center py-4 text-sm text-gray-500 dark:text-gray-400">
						{i18n.t('projComp.noMcpsMatch', { query: searchQuery })}
					</div>
				{/if}
			{:else}
				<div class="text-center py-6 bg-gray-50 dark:bg-gray-700/30 rounded-lg">
					<p class="text-gray-500 dark:text-gray-400">{i18n.t('projComp.allMcpsAssigned')}</p>
				</div>
			{/if}
		</div>
	{:else if activeTab === 'skills'}
		{#if inheritedGlobalSkills.length > 0}
			<div>
				<div class="flex items-center justify-between mb-3">
					<h3 class="text-sm font-medium text-gray-700 dark:text-gray-300 flex items-center gap-1.5">
						<Globe class="w-4 h-4 text-primary-500" />
						{i18n.t('projComp.globalInherited', { count: inheritedGlobalSkills.length })}
					</h3>
					<span class="text-xs text-gray-400 dark:text-gray-500">{i18n.t('projComp.activeViaGlobal')}</span>
				</div>
				<div class="space-y-2">
					{#each inheritedGlobalSkills as globalItem (globalItem.id)}
						{@const skill = skillLibrary.getSkillById(globalItem.skillId) ?? globalItem.skill}
						<div class="flex items-center justify-between gap-3 p-3 bg-primary-50/50 dark:bg-primary-900/10 border border-primary-100 dark:border-primary-800/40 rounded-lg">
							<div class="flex items-center gap-3 min-w-0 flex-1">
								<div class="w-8 h-8 rounded-lg bg-yellow-100 text-yellow-600 dark:bg-yellow-900/50 dark:text-yellow-400 flex items-center justify-center flex-shrink-0">
									<Sparkles class="w-4 h-4" />
								</div>
								<div class="min-w-0">
									<p class="font-medium text-gray-900 dark:text-white truncate">{skill.name}</p>
									{#if skill.description}
										<p class="text-xs text-gray-500 dark:text-gray-400 truncate">{skill.description}</p>
									{/if}
								</div>
							</div>
							<span class="px-2 py-0.5 text-xs font-medium rounded-full bg-primary-100 text-primary-700 dark:bg-primary-900/40 dark:text-primary-300 flex-shrink-0">
								{i18n.t('projComp.globalBadge')}
							</span>
						</div>
					{/each}
				</div>
			</div>
		{/if}

		<!-- Assigned Skills -->
		<div>
			<div class="flex items-center gap-3 mb-3">
				{#if projectSkills.length > 1}
					<button
						onclick={() => toggleSelectAll(projectSkills.map(a => a.id))}
						class="p-0.5 text-gray-400 hover:text-primary-500 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:ring-offset-1 rounded transition-colors"
						title={i18n.t(allSelected ? 'projComp.deselectAll' : 'projComp.selectAll')}
					>
						{#if allSelected}
							<SquareCheck class="w-4 h-4 text-primary-500" />
						{:else}
							<Square class="w-4 h-4" />
						{/if}
					</button>
				{/if}
				<h3 class="text-sm font-medium text-gray-700 dark:text-gray-300">
					{i18n.t('projComp.assignedSkills', { count: projectSkills.length })}
				</h3>
			</div>
			{#if projectSkills.length > 0}
				<div class="space-y-2">
					{#each projectSkills as assignment (assignment.id)}
						{@const skill = skillLibrary.getSkillById(assignment.skillId) ?? assignment.skill}
						<div class="flex items-center justify-between gap-3 p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg {selectedIds.has(assignment.id) ? 'ring-2 ring-primary-500/50' : ''}">
							<div class="flex items-center gap-3 min-w-0 flex-1">
								<button
									onclick={() => toggleSelection(assignment.id)}
									class="p-0.5 text-gray-400 hover:text-primary-500 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:ring-offset-1 rounded transition-colors flex-shrink-0"
								>
									{#if selectedIds.has(assignment.id)}
										<SquareCheck class="w-4 h-4 text-primary-500" />
									{:else}
										<Square class="w-4 h-4" />
									{/if}
								</button>
								<div class="w-8 h-8 rounded-lg bg-yellow-100 text-yellow-600 dark:bg-yellow-900/50 dark:text-yellow-400 flex items-center justify-center flex-shrink-0">
									<Sparkles class="w-4 h-4" />
								</div>
								<div class="min-w-0">
									<p class="font-medium text-gray-900 dark:text-white truncate {!assignment.isEnabled ? 'line-through opacity-50' : ''}">
										{skill.name}
									</p>
									{#if skill.description}
										<p class="text-xs text-gray-500 dark:text-gray-400 truncate">{skill.description}</p>
									{/if}
								</div>
							</div>
							<div class="flex items-center gap-3 flex-shrink-0">
								<button
									onclick={() => handleToggleSkill(assignment.id, !assignment.isEnabled)}
									class="relative inline-flex h-5 w-9 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none {assignment.isEnabled ? 'bg-primary-600' : 'bg-gray-300 dark:bg-gray-600'}"
									role="switch"
									aria-checked={assignment.isEnabled}
									title={i18n.t(assignment.isEnabled ? 'common.disable' : 'common.enable')}
								>
									<span class="pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out {assignment.isEnabled ? 'translate-x-4' : 'translate-x-0'}"></span>
								</button>
								<button
									onclick={() => handleRemoveSkill(assignment.skillId)}
									class="p-1.5 text-gray-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg transition-colors"
									title={i18n.t('projComp.removeFromProject')}
								>
									<Minus class="w-4 h-4" />
								</button>
							</div>
						</div>
					{/each}
				</div>
			{:else}
				<div class="text-center py-6 bg-gray-50 dark:bg-gray-700/30 rounded-lg">
					<p class="text-gray-500 dark:text-gray-400">{i18n.t('projComp.noSkillsAssigned')}</p>
					<p class="text-sm text-gray-400 dark:text-gray-500 mt-1">{i18n.t('projComp.addSkillsHint')}</p>
				</div>
			{/if}
		</div>

		<!-- Available Skills -->
		<div>
			<div class="flex items-center justify-between mb-3">
				<h3 class="text-sm font-medium text-gray-700 dark:text-gray-300">
					{i18n.t('projComp.availableSkills', { count: availableSkills.length })}
				</h3>
				{#if availableSkills.length > 3}
					<div class="relative w-48">
						<Search class="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-gray-400" />
						<input
							type="text"
							bind:value={searchQuery}
							placeholder={getSearchPlaceholder()}
							class="input pl-8 py-1.5 text-sm"
						/>
					</div>
				{/if}
			</div>
			{#if availableSkills.length > 0}
				{#if filteredAvailableSkills.length > 0}
					<div class="space-y-2">
						{#each filteredAvailableSkills as skill (skill.id)}
							<div class="flex items-center justify-between gap-3 p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
								<div class="flex items-center gap-3 min-w-0 flex-1">
									<div class="w-8 h-8 rounded-lg bg-yellow-100 text-yellow-600 dark:bg-yellow-900/50 dark:text-yellow-400 flex items-center justify-center flex-shrink-0">
										<Sparkles class="w-4 h-4" />
									</div>
									<div class="min-w-0">
										<p class="font-medium text-gray-900 dark:text-white truncate">{skill.name}</p>
										{#if skill.description}
											<p class="text-xs text-gray-500 dark:text-gray-400 truncate">{skill.description}</p>
										{/if}
									</div>
								</div>
								<button
									onclick={() => handleAddSkill(skill)}
									class="p-1.5 text-gray-400 hover:text-green-500 hover:bg-green-50 dark:hover:bg-green-900/20 rounded-lg transition-colors flex-shrink-0"
									title={i18n.t('projComp.addToProject')}
								>
									<Plus class="w-4 h-4" />
								</button>
							</div>
						{/each}
					</div>
				{:else}
					<div class="text-center py-4 text-sm text-gray-500 dark:text-gray-400">
						{i18n.t('projComp.noSkillsMatch', { query: searchQuery })}
					</div>
				{/if}
			{:else}
				<div class="text-center py-6 bg-gray-50 dark:bg-gray-700/30 rounded-lg">
					<p class="text-gray-500 dark:text-gray-400">{i18n.t('projComp.allSkillsAssigned')}</p>
				</div>
			{/if}
		</div>
	{:else if activeTab === 'agents'}
		{#if inheritedGlobalSubAgents.length > 0}
			<div>
				<div class="flex items-center justify-between mb-3">
					<h3 class="text-sm font-medium text-gray-700 dark:text-gray-300 flex items-center gap-1.5">
						<Globe class="w-4 h-4 text-primary-500" />
						{i18n.t('projComp.globalInherited', { count: inheritedGlobalSubAgents.length })}
					</h3>
					<span class="text-xs text-gray-400 dark:text-gray-500">{i18n.t('projComp.activeViaGlobal')}</span>
				</div>
				<div class="space-y-2">
					{#each inheritedGlobalSubAgents as globalItem (globalItem.id)}
						{@const agent = subagentLibrary.getSubAgentById(globalItem.subagentId) ?? globalItem.subagent}
						<div class="flex items-center justify-between gap-3 p-3 bg-primary-50/50 dark:bg-primary-900/10 border border-primary-100 dark:border-primary-800/40 rounded-lg">
							<div class="flex items-center gap-3 min-w-0 flex-1">
								<div class="w-8 h-8 rounded-lg bg-cyan-100 text-cyan-600 dark:bg-cyan-900/50 dark:text-cyan-400 flex items-center justify-center flex-shrink-0">
									<Bot class="w-4 h-4" />
								</div>
								<div class="min-w-0">
									<p class="font-medium text-gray-900 dark:text-white truncate">{agent.name}</p>
									{#if agent.description}
										<p class="text-xs text-gray-500 dark:text-gray-400 truncate">{agent.description}</p>
									{/if}
								</div>
							</div>
							<span class="px-2 py-0.5 text-xs font-medium rounded-full bg-primary-100 text-primary-700 dark:bg-primary-900/40 dark:text-primary-300 flex-shrink-0">
								{i18n.t('projComp.globalBadge')}
							</span>
						</div>
					{/each}
				</div>
			</div>
		{/if}

		<!-- Assigned SubAgents -->
		<div>
			<div class="flex items-center gap-3 mb-3">
				{#if projectSubAgents.length > 1}
					<button
						onclick={() => toggleSelectAll(projectSubAgents.map(a => a.id))}
						class="p-0.5 text-gray-400 hover:text-primary-500 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:ring-offset-1 rounded transition-colors"
						title={i18n.t(allSelected ? 'projComp.deselectAll' : 'projComp.selectAll')}
					>
						{#if allSelected}
							<SquareCheck class="w-4 h-4 text-primary-500" />
						{:else}
							<Square class="w-4 h-4" />
						{/if}
					</button>
				{/if}
				<h3 class="text-sm font-medium text-gray-700 dark:text-gray-300">
					{i18n.t('projComp.assignedAgents', { count: projectSubAgents.length })}
				</h3>
			</div>
			{#if projectSubAgents.length > 0}
				<div class="space-y-2">
					{#each projectSubAgents as assignment (assignment.id)}
						{@const agent = subagentLibrary.getSubAgentById(assignment.subagentId) ?? assignment.subagent}
						<div class="flex items-center justify-between gap-3 p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg {selectedIds.has(assignment.id) ? 'ring-2 ring-primary-500/50' : ''}">
							<div class="flex items-center gap-3 min-w-0 flex-1">
								<button
									onclick={() => toggleSelection(assignment.id)}
									class="p-0.5 text-gray-400 hover:text-primary-500 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:ring-offset-1 rounded transition-colors flex-shrink-0"
								>
									{#if selectedIds.has(assignment.id)}
										<SquareCheck class="w-4 h-4 text-primary-500" />
									{:else}
										<Square class="w-4 h-4" />
									{/if}
								</button>
								<div class="w-8 h-8 rounded-lg bg-cyan-100 text-cyan-600 dark:bg-cyan-900/50 dark:text-cyan-400 flex items-center justify-center flex-shrink-0">
									<Bot class="w-4 h-4" />
								</div>
								<div class="min-w-0">
									<p class="font-medium text-gray-900 dark:text-white truncate {!assignment.isEnabled ? 'line-through opacity-50' : ''}">
										{agent.name}
									</p>
									{#if agent.description}
										<p class="text-xs text-gray-500 dark:text-gray-400 truncate">{agent.description}</p>
									{/if}
								</div>
							</div>
							<div class="flex items-center gap-3 flex-shrink-0">
								<button
									onclick={() => handleToggleSubAgent(assignment.id, !assignment.isEnabled)}
									class="relative inline-flex h-5 w-9 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none {assignment.isEnabled ? 'bg-primary-600' : 'bg-gray-300 dark:bg-gray-600'}"
									role="switch"
									aria-checked={assignment.isEnabled}
									title={i18n.t(assignment.isEnabled ? 'common.disable' : 'common.enable')}
								>
									<span class="pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out {assignment.isEnabled ? 'translate-x-4' : 'translate-x-0'}"></span>
								</button>
								<button
									onclick={() => handleRemoveSubAgent(assignment.subagentId)}
									class="p-1.5 text-gray-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg transition-colors"
									title={i18n.t('projComp.removeFromProject')}
								>
									<Minus class="w-4 h-4" />
								</button>
							</div>
						</div>
					{/each}
				</div>
			{:else}
				<div class="text-center py-6 bg-gray-50 dark:bg-gray-700/30 rounded-lg">
					<p class="text-gray-500 dark:text-gray-400">{i18n.t('projComp.noAgentsAssigned')}</p>
					<p class="text-sm text-gray-400 dark:text-gray-500 mt-1">{i18n.t('projComp.addAgentsHint')}</p>
				</div>
			{/if}
		</div>

		<!-- Available SubAgents -->
		<div>
			<div class="flex items-center justify-between mb-3">
				<h3 class="text-sm font-medium text-gray-700 dark:text-gray-300">
					{i18n.t('projComp.availableAgents', { count: availableSubAgents.length })}
				</h3>
				{#if availableSubAgents.length > 3}
					<div class="relative w-48">
						<Search class="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-gray-400" />
						<input
							type="text"
							bind:value={searchQuery}
							placeholder={getSearchPlaceholder()}
							class="input pl-8 py-1.5 text-sm"
						/>
					</div>
				{/if}
			</div>
			{#if availableSubAgents.length > 0}
				{#if filteredAvailableSubAgents.length > 0}
					<div class="space-y-2">
						{#each filteredAvailableSubAgents as agent (agent.id)}
							<div class="flex items-center justify-between gap-3 p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
								<div class="flex items-center gap-3 min-w-0 flex-1">
									<div class="w-8 h-8 rounded-lg bg-cyan-100 text-cyan-600 dark:bg-cyan-900/50 dark:text-cyan-400 flex items-center justify-center flex-shrink-0">
										<Bot class="w-4 h-4" />
									</div>
									<div class="min-w-0">
										<p class="font-medium text-gray-900 dark:text-white truncate">{agent.name}</p>
										{#if agent.description}
											<p class="text-xs text-gray-500 dark:text-gray-400 truncate">{agent.description}</p>
										{/if}
									</div>
								</div>
								<button
									onclick={() => handleAddSubAgent(agent)}
									class="p-1.5 text-gray-400 hover:text-green-500 hover:bg-green-50 dark:hover:bg-green-900/20 rounded-lg transition-colors flex-shrink-0"
									title={i18n.t('projComp.addToProject')}
								>
									<Plus class="w-4 h-4" />
								</button>
							</div>
						{/each}
					</div>
				{:else}
					<div class="text-center py-4 text-sm text-gray-500 dark:text-gray-400">
						{i18n.t('projComp.noAgentsMatch', { query: searchQuery })}
					</div>
				{/if}
			{:else}
				<div class="text-center py-6 bg-gray-50 dark:bg-gray-700/30 rounded-lg">
					<p class="text-gray-500 dark:text-gray-400">{i18n.t('projComp.allAgentsAssigned')}</p>
				</div>
			{/if}
		</div>
	{:else if activeTab === 'commands'}
		{#if inheritedGlobalCommands.length > 0}
			<div>
				<div class="flex items-center justify-between mb-3">
					<h3 class="text-sm font-medium text-gray-700 dark:text-gray-300 flex items-center gap-1.5">
						<Globe class="w-4 h-4 text-primary-500" />
						{i18n.t('projComp.globalInherited', { count: inheritedGlobalCommands.length })}
					</h3>
					<span class="text-xs text-gray-400 dark:text-gray-500">{i18n.t('projComp.activeViaGlobal')}</span>
				</div>
				<div class="space-y-2">
					{#each inheritedGlobalCommands as globalItem (globalItem.id)}
						{@const command = commandLibrary.getCommandById(globalItem.commandId) ?? globalItem.command}
						<div class="flex items-center justify-between gap-3 p-3 bg-primary-50/50 dark:bg-primary-900/10 border border-primary-100 dark:border-primary-800/40 rounded-lg">
							<div class="flex items-center gap-3 min-w-0 flex-1">
								<div class="w-8 h-8 rounded-lg bg-indigo-100 text-indigo-600 dark:bg-indigo-900/50 dark:text-indigo-400 flex items-center justify-center flex-shrink-0">
									<Terminal class="w-4 h-4" />
								</div>
								<div class="min-w-0">
									<p class="font-medium text-gray-900 dark:text-white truncate">/{command.name}</p>
									{#if command.description}
										<p class="text-xs text-gray-500 dark:text-gray-400 truncate">{command.description}</p>
									{/if}
								</div>
							</div>
							<span class="px-2 py-0.5 text-xs font-medium rounded-full bg-primary-100 text-primary-700 dark:bg-primary-900/40 dark:text-primary-300 flex-shrink-0">
								{i18n.t('projComp.globalBadge')}
							</span>
						</div>
					{/each}
				</div>
			</div>
		{/if}

		<!-- Assigned Commands -->
		<div>
			<div class="flex items-center gap-3 mb-3">
				{#if projectCommands.length > 1}
					<button
						onclick={() => toggleSelectAll(projectCommands.map(a => a.id))}
						class="p-0.5 text-gray-400 hover:text-primary-500 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:ring-offset-1 rounded transition-colors"
						title={i18n.t(allSelected ? 'projComp.deselectAll' : 'projComp.selectAll')}
					>
						{#if allSelected}
							<SquareCheck class="w-4 h-4 text-primary-500" />
						{:else}
							<Square class="w-4 h-4" />
						{/if}
					</button>
				{/if}
				<h3 class="text-sm font-medium text-gray-700 dark:text-gray-300">
					{i18n.t('projComp.assignedCommands', { count: projectCommands.length })}
				</h3>
			</div>
			{#if projectCommands.length > 0}
				<div class="space-y-2">
					{#each projectCommands as assignment (assignment.id)}
						{@const command = commandLibrary.getCommandById(assignment.commandId) ?? assignment.command}
						<div class="flex items-center justify-between gap-3 p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg {selectedIds.has(assignment.id) ? 'ring-2 ring-primary-500/50' : ''}">
							<div class="flex items-center gap-3 min-w-0 flex-1">
								<button
									onclick={() => toggleSelection(assignment.id)}
									class="p-0.5 text-gray-400 hover:text-primary-500 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:ring-offset-1 rounded transition-colors flex-shrink-0"
								>
									{#if selectedIds.has(assignment.id)}
										<SquareCheck class="w-4 h-4 text-primary-500" />
									{:else}
										<Square class="w-4 h-4" />
									{/if}
								</button>
								<div class="w-8 h-8 rounded-lg bg-indigo-100 text-indigo-600 dark:bg-indigo-900/50 dark:text-indigo-400 flex items-center justify-center flex-shrink-0">
									<Terminal class="w-4 h-4" />
								</div>
								<div class="min-w-0">
									<p class="font-medium text-gray-900 dark:text-white truncate {!assignment.isEnabled ? 'line-through opacity-50' : ''}">
										/{command.name}
									</p>
									{#if command.description}
										<p class="text-xs text-gray-500 dark:text-gray-400 truncate">{command.description}</p>
									{/if}
								</div>
							</div>
							<div class="flex items-center gap-3 flex-shrink-0">
								<button
									onclick={() => handleToggleCommand(assignment.id, !assignment.isEnabled)}
									class="relative inline-flex h-5 w-9 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none {assignment.isEnabled ? 'bg-primary-600' : 'bg-gray-300 dark:bg-gray-600'}"
									role="switch"
									aria-checked={assignment.isEnabled}
									title={i18n.t(assignment.isEnabled ? 'common.disable' : 'common.enable')}
								>
									<span class="pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out {assignment.isEnabled ? 'translate-x-4' : 'translate-x-0'}"></span>
								</button>
								<button
									onclick={() => handleRemoveCommand(assignment.commandId)}
									class="p-1.5 text-gray-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg transition-colors"
									title={i18n.t('projComp.removeFromProject')}
								>
									<Minus class="w-4 h-4" />
								</button>
							</div>
						</div>
					{/each}
				</div>
			{:else}
				<div class="text-center py-6 bg-gray-50 dark:bg-gray-700/30 rounded-lg">
					<p class="text-gray-500 dark:text-gray-400">{i18n.t('projComp.noCommandsAssigned')}</p>
					<p class="text-sm text-gray-400 dark:text-gray-500 mt-1">{i18n.t('projComp.addCommandsHint')}</p>
				</div>
			{/if}
		</div>

		<!-- Available Commands -->
		<div>
			<div class="flex items-center justify-between mb-3">
				<h3 class="text-sm font-medium text-gray-700 dark:text-gray-300">
					{i18n.t('projComp.availableCommands', { count: availableCommands.length })}
				</h3>
				{#if availableCommands.length > 3}
					<div class="relative w-48">
						<Search class="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-gray-400" />
						<input
							type="text"
							bind:value={searchQuery}
							placeholder={getSearchPlaceholder()}
							class="input pl-8 py-1.5 text-sm"
						/>
					</div>
				{/if}
			</div>
			{#if availableCommands.length > 0}
				{#if filteredAvailableCommands.length > 0}
					<div class="space-y-2">
						{#each filteredAvailableCommands as command (command.id)}
							<div class="flex items-center justify-between gap-3 p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
								<div class="flex items-center gap-3 min-w-0 flex-1">
									<div class="w-8 h-8 rounded-lg bg-indigo-100 text-indigo-600 dark:bg-indigo-900/50 dark:text-indigo-400 flex items-center justify-center flex-shrink-0">
										<Terminal class="w-4 h-4" />
									</div>
									<div class="min-w-0">
										<p class="font-medium text-gray-900 dark:text-white truncate">/{command.name}</p>
										{#if command.description}
											<p class="text-xs text-gray-500 dark:text-gray-400 truncate">{command.description}</p>
										{/if}
									</div>
								</div>
								<button
									onclick={() => handleAddCommand(command)}
									class="p-1.5 text-gray-400 hover:text-green-500 hover:bg-green-50 dark:hover:bg-green-900/20 rounded-lg transition-colors flex-shrink-0"
									title={i18n.t('projComp.addToProject')}
								>
									<Plus class="w-4 h-4" />
								</button>
							</div>
						{/each}
					</div>
				{:else}
					<div class="text-center py-4 text-sm text-gray-500 dark:text-gray-400">
						{i18n.t('projComp.noCommandsMatch', { query: searchQuery })}
					</div>
				{/if}
			{:else}
				<div class="text-center py-6 bg-gray-50 dark:bg-gray-700/30 rounded-lg">
					<p class="text-gray-500 dark:text-gray-400">{i18n.t('projComp.allCommandsAssigned')}</p>
				</div>
			{/if}
		</div>
	{:else if activeTab === 'hooks'}
		{#if inheritedGlobalHooks.length > 0}
			<div>
				<div class="flex items-center justify-between mb-3">
					<h3 class="text-sm font-medium text-gray-700 dark:text-gray-300 flex items-center gap-1.5">
						<Globe class="w-4 h-4 text-primary-500" />
						{i18n.t('projComp.globalInherited', { count: inheritedGlobalHooks.length })}
					</h3>
					<span class="text-xs text-gray-400 dark:text-gray-500">{i18n.t('projComp.activeViaGlobal')}</span>
				</div>
				<div class="space-y-2">
					{#each inheritedGlobalHooks as globalItem (globalItem.id)}
						{@const hook = hookLibrary.getHookById(globalItem.hookId) ?? globalItem.hook}
						<div class="flex items-center justify-between gap-3 p-3 bg-primary-50/50 dark:bg-primary-900/10 border border-primary-100 dark:border-primary-800/40 rounded-lg">
							<div class="flex items-center gap-3 min-w-0 flex-1">
								<div class="w-8 h-8 rounded-lg bg-orange-100 text-orange-600 dark:bg-orange-900/50 dark:text-orange-400 flex items-center justify-center flex-shrink-0">
									<Zap class="w-4 h-4" />
								</div>
								<div class="min-w-0">
									<p class="font-medium text-gray-900 dark:text-white truncate">{hook.name}</p>
									<p class="text-xs text-gray-500 dark:text-gray-400 truncate">
										{getEventLabel(hook.eventType)}{#if hook.matcher} &middot; {hook.matcher}{/if}
									</p>
								</div>
							</div>
							<span class="px-2 py-0.5 text-xs font-medium rounded-full bg-primary-100 text-primary-700 dark:bg-primary-900/40 dark:text-primary-300 flex-shrink-0">
								{i18n.t('projComp.globalBadge')}
							</span>
						</div>
					{/each}
				</div>
			</div>
		{/if}

		<!-- Assigned Hooks -->
		<div>
			<div class="flex items-center gap-3 mb-3">
				{#if projectHooks.length > 1}
					<button
						onclick={() => toggleSelectAll(projectHooks.map(a => a.id))}
						class="p-0.5 text-gray-400 hover:text-primary-500 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:ring-offset-1 rounded transition-colors"
						title={i18n.t(allSelected ? 'projComp.deselectAll' : 'projComp.selectAll')}
					>
						{#if allSelected}
							<SquareCheck class="w-4 h-4 text-primary-500" />
						{:else}
							<Square class="w-4 h-4" />
						{/if}
					</button>
				{/if}
				<h3 class="text-sm font-medium text-gray-700 dark:text-gray-300">
					{i18n.t('projComp.assignedHooks', { count: projectHooks.length })}
				</h3>
			</div>
			{#if projectHooks.length > 0}
				<div class="space-y-2">
					{#each projectHooks as assignment (assignment.id)}
						{@const hook = hookLibrary.getHookById(assignment.hookId) ?? assignment.hook}
						<div class="flex items-center justify-between gap-3 p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg {selectedIds.has(assignment.id) ? 'ring-2 ring-primary-500/50' : ''}">
							<div class="flex items-center gap-3 min-w-0 flex-1">
								<button
									onclick={() => toggleSelection(assignment.id)}
									class="p-0.5 text-gray-400 hover:text-primary-500 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:ring-offset-1 rounded transition-colors flex-shrink-0"
								>
									{#if selectedIds.has(assignment.id)}
										<SquareCheck class="w-4 h-4 text-primary-500" />
									{:else}
										<Square class="w-4 h-4" />
									{/if}
								</button>
								<div class="w-8 h-8 rounded-lg bg-orange-100 text-orange-600 dark:bg-orange-900/50 dark:text-orange-400 flex items-center justify-center flex-shrink-0">
									<Zap class="w-4 h-4" />
								</div>
								<div class="min-w-0">
									<p class="font-medium text-gray-900 dark:text-white truncate {!assignment.isEnabled ? 'line-through opacity-50' : ''}">
										{hook.name}
									</p>
									<p class="text-xs text-gray-500 dark:text-gray-400 truncate">
										{getEventLabel(hook.eventType)}{#if hook.matcher} &middot; {hook.matcher}{/if}
									</p>
								</div>
							</div>
							<div class="flex items-center gap-3 flex-shrink-0">
								<button
									onclick={() => handleToggleHook(assignment.id, !assignment.isEnabled)}
									class="relative inline-flex h-5 w-9 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none {assignment.isEnabled ? 'bg-primary-600' : 'bg-gray-300 dark:bg-gray-600'}"
									role="switch"
									aria-checked={assignment.isEnabled}
									title={i18n.t(assignment.isEnabled ? 'common.disable' : 'common.enable')}
								>
									<span class="pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out {assignment.isEnabled ? 'translate-x-4' : 'translate-x-0'}"></span>
								</button>
								<button
									onclick={() => handleRemoveHook(assignment.hookId)}
									class="p-1.5 text-gray-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg transition-colors"
									title={i18n.t('projComp.removeFromProject')}
								>
									<Minus class="w-4 h-4" />
								</button>
							</div>
						</div>
					{/each}
				</div>
			{:else}
				<div class="text-center py-6 bg-gray-50 dark:bg-gray-700/30 rounded-lg">
					<p class="text-gray-500 dark:text-gray-400">{i18n.t('projComp.noHooksAssigned')}</p>
					<p class="text-sm text-gray-400 dark:text-gray-500 mt-1">{i18n.t('projComp.addHooksHint')}</p>
				</div>
			{/if}
		</div>

		<!-- Available Hooks -->
		<div>
			<div class="flex items-center justify-between mb-3">
				<h3 class="text-sm font-medium text-gray-700 dark:text-gray-300">
					{i18n.t('projComp.availableHooks', { count: availableHooks.length })}
				</h3>
				{#if availableHooks.length > 3}
					<div class="relative w-48">
						<Search class="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-gray-400" />
						<input
							type="text"
							bind:value={searchQuery}
							placeholder={getSearchPlaceholder()}
							class="input pl-8 py-1.5 text-sm"
						/>
					</div>
				{/if}
			</div>
			{#if availableHooks.length > 0}
				{#if filteredAvailableHooks.length > 0}
					<div class="space-y-2">
						{#each filteredAvailableHooks as hook (hook.id)}
							<div class="flex items-center justify-between gap-3 p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
								<div class="flex items-center gap-3 min-w-0 flex-1">
									<div class="w-8 h-8 rounded-lg bg-orange-100 text-orange-600 dark:bg-orange-900/50 dark:text-orange-400 flex items-center justify-center flex-shrink-0">
										<Zap class="w-4 h-4" />
									</div>
									<div class="min-w-0">
										<p class="font-medium text-gray-900 dark:text-white truncate">{hook.name}</p>
										<p class="text-xs text-gray-500 dark:text-gray-400 truncate">
											{getEventLabel(hook.eventType)}{#if hook.matcher} &middot; {hook.matcher}{/if}
										</p>
									</div>
								</div>
								<button
									onclick={() => handleAddHook(hook)}
									class="p-1.5 text-gray-400 hover:text-green-500 hover:bg-green-50 dark:hover:bg-green-900/20 rounded-lg transition-colors flex-shrink-0"
									title={i18n.t('projComp.addToProject')}
								>
									<Plus class="w-4 h-4" />
								</button>
							</div>
						{/each}
					</div>
				{:else}
					<div class="text-center py-4 text-sm text-gray-500 dark:text-gray-400">
						{i18n.t('projComp.noHooksMatch', { query: searchQuery })}
					</div>
				{/if}
			{:else}
				<div class="text-center py-6 bg-gray-50 dark:bg-gray-700/30 rounded-lg">
					<p class="text-gray-500 dark:text-gray-400">{i18n.t('projComp.allHooksAssigned')}</p>
				</div>
			{/if}
		</div>
	{/if}
</div>
