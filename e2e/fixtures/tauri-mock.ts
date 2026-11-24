import type { Page } from '@playwright/test';

/**
 * Mock data for E2E tests
 */
export const mockData = {
	mcps: [
		{
			id: 1,
			name: 'filesystem',
			description: 'Access to local filesystem operations',
			type: 'stdio',
			command: 'npx',
			args: ['-y', '@anthropic-ai/mcp-server-filesystem', '/Users'],
			url: null,
			headers: null,
			env: null,
			icon: null,
			tags: ['file', 'storage'],
			source: 'manual',
			sourcePath: null,
			isEnabledGlobal: true,
			createdAt: '2024-01-01T00:00:00Z',
			updatedAt: '2024-01-01T00:00:00Z'
		},
		{
			id: 2,
			name: 'github',
			description: 'GitHub API integration',
			type: 'stdio',
			command: 'npx',
			args: ['-y', '@anthropic-ai/mcp-server-github'],
			url: null,
			headers: null,
			env: { GITHUB_TOKEN: 'test-token' },
			icon: null,
			tags: ['git', 'api'],
			source: 'manual',
			sourcePath: null,
			isEnabledGlobal: false,
			createdAt: '2024-01-02T00:00:00Z',
			updatedAt: '2024-01-02T00:00:00Z'
		},
		{
			id: 3,
			name: 'http-api',
			description: 'HTTP API server',
			type: 'http',
			command: null,
			args: null,
			url: 'https://api.example.com/mcp',
			headers: { Authorization: 'Bearer test' },
			env: null,
			icon: null,
			tags: ['api', 'http'],
			source: 'manual',
			sourcePath: null,
			isEnabledGlobal: false,
			createdAt: '2024-01-03T00:00:00Z',
			updatedAt: '2024-01-03T00:00:00Z'
		}
	],
	projects: [
		{
			id: 1,
			name: 'test-project',
			path: '/Users/test/projects/test-project',
			hasMcpFile: true,
			hasSettingsFile: true,
			lastScannedAt: '2024-01-01T00:00:00Z',
			editorType: 'claude_code',
			createdAt: '2024-01-01T00:00:00Z',
			updatedAt: '2024-01-01T00:00:00Z',
			assignedMcps: [
				{
					id: 1,
					mcpId: 1,
					mcp: null, // Will be populated in handler
					isEnabled: true,
					envOverrides: null,
					displayOrder: 0
				}
			]
		},
		{
			id: 2,
			name: 'another-project',
			path: '/Users/test/projects/another-project',
			hasMcpFile: false,
			hasSettingsFile: false,
			lastScannedAt: null,
			editorType: 'claude_code',
			createdAt: '2024-01-02T00:00:00Z',
			updatedAt: '2024-01-02T00:00:00Z',
			assignedMcps: []
		}
	],
	globalMcps: [
		{
			id: 1,
			mcpId: 1,
			mcp: null, // Will be populated in handler
			isEnabled: true,
			envOverrides: null
		}
	],
	skills: [
		{
			id: 1,
			name: 'commit',
			type: 'command',
			description: 'Create a git commit with a descriptive message',
			content: 'Create a commit with a descriptive message...',
			source: 'manual',
			sourcePath: null,
			createdAt: '2024-01-01T00:00:00Z',
			updatedAt: '2024-01-01T00:00:00Z'
		}
	],
	globalSkills: [],
	subagents: [
		{
			id: 1,
			name: 'code-reviewer',
			type: 'agent',
			description: 'Reviews code for issues and improvements',
			content: 'Review the code and provide feedback...',
			model: 'sonnet',
			source: 'manual',
			sourcePath: null,
			createdAt: '2024-01-01T00:00:00Z',
			updatedAt: '2024-01-01T00:00:00Z'
		}
	],
	globalSubAgents: [],
	hooks: [],
	repos: [],
	profiles: [
		{
			id: 1,
			name: 'default-profile',
			description: 'Default profile configuration',
			icon: null,
			isActive: true,
			createdAt: '2024-01-01T00:00:00Z',
			updatedAt: '2024-01-01T00:00:00Z'
		}
	],
	activeProfile: {
		id: 1,
		name: 'default-profile',
		description: 'Default profile configuration',
		icon: null,
		isActive: true,
		createdAt: '2024-01-01T00:00:00Z',
		updatedAt: '2024-01-01T00:00:00Z'
	},
	statuslines: [] as any[],
	activeStatusLine: null as any,
	commands: [
		{
			id: 1,
			name: 'test-command',
			description: 'A test command',
			content: 'echo hello',
			source: 'manual',
			isFavorite: false,
			createdAt: '2024-01-01T00:00:00Z',
			updatedAt: '2024-01-01T00:00:00Z'
		}
	],
	appSettings: {
		defaultEditor: 'claude_code'
	}
};

/**
 * Inject Tauri API mocks into the page
 */
export async function mockTauriApi(page: Page) {
	await page.addInitScript(() => {
		// The app defaults to pt-BR; the specs assert the English copy
		// (a later init script may set another locale).
		try {
			localStorage.setItem('locale', 'en');
		} catch {
			// storage unavailable
		}

		// Mock data (will be available as window.__MOCK_DATA__)
		const mockData = {
			mcps: [
				{
					id: 1,
					name: 'filesystem',
					description: 'Access to local filesystem operations',
					type: 'stdio',
					command: 'npx',
					args: ['-y', '@anthropic-ai/mcp-server-filesystem', '/Users'],
					url: null,
					headers: null,
					env: null,
					icon: null,
					tags: ['file', 'storage'],
					source: 'manual',
					sourcePath: null,
					isEnabledGlobal: true,
					createdAt: '2024-01-01T00:00:00Z',
					updatedAt: '2024-01-01T00:00:00Z'
				},
				{
					id: 2,
					name: 'github',
					description: 'GitHub API integration',
					type: 'stdio',
					command: 'npx',
					args: ['-y', '@anthropic-ai/mcp-server-github'],
					url: null,
					headers: null,
					env: { GITHUB_TOKEN: 'test-token' },
					icon: null,
					tags: ['git', 'api'],
					source: 'manual',
					sourcePath: null,
					isEnabledGlobal: false,
					createdAt: '2024-01-02T00:00:00Z',
					updatedAt: '2024-01-02T00:00:00Z'
				},
				{
					id: 3,
					name: 'http-api',
					description: 'HTTP API server',
					type: 'http',
					command: null,
					args: null,
					url: 'https://api.example.com/mcp',
					headers: { Authorization: 'Bearer test' },
					env: null,
					icon: null,
					tags: ['api', 'http'],
					source: 'manual',
					sourcePath: null,
					isEnabledGlobal: false,
					createdAt: '2024-01-03T00:00:00Z',
					updatedAt: '2024-01-03T00:00:00Z'
				}
			],
			projects: [
				{
					id: 1,
					name: 'test-project',
					path: '/Users/test/projects/test-project',
					hasMcpFile: true,
					hasSettingsFile: true,
					lastScannedAt: '2024-01-01T00:00:00Z',
					editorType: 'claude_code',
					createdAt: '2024-01-01T00:00:00Z',
					updatedAt: '2024-01-01T00:00:00Z',
					assignedMcps: []
				},
				{
					id: 2,
					name: 'another-project',
					path: '/Users/test/projects/another-project',
					hasMcpFile: false,
					hasSettingsFile: false,
					lastScannedAt: null,
					editorType: 'claude_code',
					createdAt: '2024-01-02T00:00:00Z',
					updatedAt: '2024-01-02T00:00:00Z',
					assignedMcps: []
				}
			],
			globalMcps: [],
			skills: [
				{
					id: 1,
					name: 'commit',
					type: 'command',
					description: 'Create a git commit with a descriptive message',
					content: 'Create a commit with a descriptive message...',
					source: 'manual',
					sourcePath: null,
					createdAt: '2024-01-01T00:00:00Z',
					updatedAt: '2024-01-01T00:00:00Z'
				}
			],
			globalSkills: [],
			subagents: [
				{
					id: 1,
					name: 'code-reviewer',
					type: 'agent',
					description: 'Reviews code for issues and improvements',
					content: 'Review the code and provide feedback...',
					model: 'sonnet',
					source: 'manual',
					sourcePath: null,
					createdAt: '2024-01-01T00:00:00Z',
					updatedAt: '2024-01-01T00:00:00Z'
				}
			],
			globalSubAgents: [],
			hooks: [],
			repos: [],
			profiles: [
				{
					id: 1,
					name: 'default-profile',
					description: 'Default profile configuration',
					icon: null,
					isActive: true,
					createdAt: '2024-01-01T00:00:00Z',
					updatedAt: '2024-01-01T00:00:00Z'
				}
			],
			activeProfile: {
				id: 1,
				name: 'default-profile',
				description: 'Default profile configuration',
				icon: null,
				isActive: true,
				createdAt: '2024-01-01T00:00:00Z',
				updatedAt: '2024-01-01T00:00:00Z'
			},
			statuslines: [],
			activeStatusLine: null,
			commands: [
				{
					id: 1,
					name: 'test-command',
					description: 'A test command',
					content: 'echo hello',
					source: 'manual',
					isFavorite: false,
					createdAt: '2024-01-01T00:00:00Z',
					updatedAt: '2024-01-01T00:00:00Z'
				}
			]
		};

		// Track invoke calls for assertions
		(window as any).__INVOKE_CALLS__ = [];

		// Create mock invoke function
		const mockInvoke = async (cmd: string, args?: any): Promise<any> => {
			// Log the call
			(window as any).__INVOKE_CALLS__.push({ cmd, args, timestamp: Date.now() });
			console.log(`[Tauri Mock] invoke('${cmd}')`, args);

			// Return mock data based on command
			switch (cmd) {
				// MCP commands
				case 'get_all_mcps':
					return mockData.mcps;
				case 'create_mcp':
					const newMcp = { ...args.mcp, id: Date.now(), createdAt: new Date().toISOString(), updatedAt: new Date().toISOString() };
					mockData.mcps.push(newMcp);
					return newMcp;
				case 'update_mcp':
					return { ...mockData.mcps.find(m => m.id === args.id), ...args.mcp };
				case 'delete_mcp':
					mockData.mcps = mockData.mcps.filter(m => m.id !== args.id);
					return null;
				case 'duplicate_mcp':
					const orig = mockData.mcps.find(m => m.id === args.id);
					const dup = { ...orig, id: Date.now(), name: `${orig?.name} (copy)` };
					mockData.mcps.push(dup);
					return dup;
				case 'toggle_global_mcp':
					return null;

				// Project commands
				case 'get_all_projects':
					return mockData.projects;
				case 'get_global_mcps':
					return mockData.globalMcps;
				case 'add_project':
					const newProj = { ...args.project, id: Date.now(), assignedMcps: [], createdAt: new Date().toISOString(), updatedAt: new Date().toISOString() };
					mockData.projects.push(newProj);
					return newProj;
				case 'remove_project':
					mockData.projects = mockData.projects.filter(p => p.id !== args.id);
					return null;
				case 'browse_for_project':
					return '/Users/test/projects/new-project';
				case 'assign_mcp_to_project':
				case 'remove_mcp_from_project':
				case 'toggle_project_mcp':
				case 'sync_project_config':
				case 'add_global_mcp':
				case 'remove_global_mcp':
				case 'toggle_global_mcp_assignment':
				case 'sync_global_config':
				case 'scan_claude_directory':
				case 'update_project_editor_type':
					return null;

				// Skill commands
				case 'get_all_skills':
					return mockData.skills;
				case 'get_global_skills':
					return mockData.globalSkills;
				case 'delete_skill':
				case 'add_global_skill':
				case 'remove_global_skill':
				case 'toggle_global_skill':
				case 'assign_skill_to_project':
				case 'remove_skill_from_project':
				case 'toggle_project_skill':
				case 'delete_skill_file':
					return null;

				// SubAgent commands
				case 'get_all_subagents':
					return mockData.subagents;
				case 'get_global_subagents':
					return mockData.globalSubAgents;
				case 'delete_subagent':
				case 'add_global_subagent':
				case 'remove_global_subagent':
				case 'toggle_global_subagent':
				case 'assign_subagent_to_project':
				case 'remove_subagent_from_project':
				case 'toggle_project_subagent':
					return null;

				// Hook commands
				case 'get_all_hooks':
					return mockData.hooks;
				case 'get_global_hooks':
					return [];
				case 'seed_hook_templates':
				case 'delete_hook':
				case 'add_global_hook':
				case 'remove_global_hook':
				case 'toggle_global_hook':
				case 'assign_hook_to_project':
				case 'remove_hook_from_project':
				case 'toggle_project_hook':
					return null;

				// Profile commands
				case 'get_all_profiles':
					return mockData.profiles;
				case 'get_active_profile':
					return mockData.activeProfile;
				case 'create_profile':
					return { ...args, id: Date.now(), createdAt: new Date().toISOString(), updatedAt: new Date().toISOString() };
				case 'update_profile':
				case 'delete_profile':
				case 'duplicate_profile':
				case 'activate_profile':
				case 'deactivate_profile':
					return null;

				// Status Line commands
				case 'get_all_statuslines':
					return mockData.statuslines;
				case 'get_active_statusline':
					return mockData.activeStatusLine;
				case 'create_statusline':
					return { ...args, id: Date.now(), createdAt: new Date().toISOString(), updatedAt: new Date().toISOString() };
				case 'update_statusline':
				case 'delete_statusline':
				case 'duplicate_statusline':
				case 'activate_statusline':
				case 'deactivate_statusline':
				case 'get_statusline_gallery':
				case 'import_statusline_from_gallery':
					return null;

				// Command commands
				case 'get_all_commands':
					return mockData.commands;
				case 'create_command':
					return { ...args, id: Date.now(), createdAt: new Date().toISOString(), updatedAt: new Date().toISOString() };
				case 'update_command':
				case 'delete_command':
				case 'duplicate_command':
					return null;

				// Sound commands
				case 'get_all_sounds':
					return [];
				case 'get_all_sound_notifications':
					return [];
				case 'play_sound':
				case 'stop_sound':
					return null;

				// Repo/marketplace commands
				case 'get_all_repos':
					return mockData.repos;
				case 'seed_default_repos':
				case 'remove_repo':
				case 'toggle_repo':
				case 'reset_repos_to_defaults':
					return null;

				// Settings commands
				case 'get_app_settings':
					return { defaultEditor: 'claude_code' };
				case 'update_app_settings':
				case 'open_config_file':
				case 'backup_configs':
					return null;

				// Debug commands
				case 'enable_debug_mode':
					return '/path/to/debug.log';
				case 'disable_debug_mode':
				case 'is_debug_mode_enabled':
					return false;
				case 'get_debug_log_path':
					return null;
				case 'open_logs_folder':
				case 'write_frontend_log':
				case 'write_invoke_log':
					return null;

				// Claude JSON commands
				case 'toggle_mcp_in_claude_json':
				case 'remove_mcp_from_claude_json':
				case 'remove_global_mcp_from_claude_json':
					return null;

				// MCP testing
				case 'test_mcp':
				case 'test_mcp_config':
					return {
						success: true,
						serverInfo: { name: 'test-server', version: '1.0.0' },
						tools: [{ name: 'test-tool', description: 'A test tool', inputSchema: null }],
						resourcesSupported: false,
						promptsSupported: false,
						error: null,
						responseTimeMs: 100
					};

				// What's new
				case 'get_whats_new_for_version':
					return null;
				case 'mark_whats_new_seen':
				case 'get_last_seen_version':
					return null;

				// Multi-CLI sync (src-tauri/src/commands/sync.rs)
				case 'get_mcp_management_status':
					return { externallyManaged: true, agentsDir: '/home/test/.agents', globalSource: '/home/test/.agents/mcp/global.json', syncToolAvailable: true, message: null };
				case 'get_sync_health': {
					const on = { state: 'on' };
					return {
						inSync: false,
						resources: 14,
						checkedAt: new Date().toISOString(),
						missingSecrets: ['AGY_FIGMA_TOKEN'],
						secretsPermissionsOk: true,
						cliVersions: [
							{ cli: 'claude', installed: '2.1.331', tested: '2.x', known: true, message: null },
							{ cli: 'codex', installed: '0.171.0', tested: '0.160 a 0.179', known: true, message: null },
							{ cli: 'agy', installed: '1.5.16', tested: '1.5.x', known: true, message: null }
						],
						report: [
							{ check: 'global', status: 'warn', summary: 'global: 8 MCPs; fora de sincronia: agy:figma, claude:icecat', detail: null },
							{ check: 'projeto.site', status: 'ok', summary: 'site: Claude, Codex e agy iguais', detail: null }
						],
						divergences: [
							{ kind: 'drift', cli: 'agy', scope: { type: 'global' }, name: 'comfyui', filePath: '/home/test/.gemini/config/mcp_config.json', message: 'comfyui: o Antigravity CLI (global) está diferente da fonte', actions: ['reapply'] },
							{ kind: 'missingInCli', cli: 'claude', scope: { type: 'global' }, name: 'icecat', filePath: '/home/test/.claude.json', message: 'icecat: está na fonte mas falta no Claude Code (global)', actions: ['reapply'] },
							{ kind: 'onlyInCli', cli: 'agy', scope: { type: 'global' }, name: 'figma', filePath: '/home/test/.gemini/config/mcp_config.json', message: 'figma: está no Antigravity CLI (global) mas não na fonte', actions: ['import', 'reapply'] },
							{ kind: 'claudeLocalScope', cli: 'claude', scope: { type: 'project', path: '/home/test/w/site' }, name: 'hostinger', filePath: '/home/test/.claude.json', message: 'hostinger: cópia antiga no escopo local do Claude vence a do projeto (site)', actions: ['removeLocalCopy'] }
						]
					};
				}
				case 'get_sync_resources': {
					const t = (c: string, x: string, a: string) => ({ claude: { state: c }, codex: { state: x }, agy: { state: a } });
					const r = (name: string, extra: any) => ({ uid: `mcp:global:${name}`, kind: 'mcp', name, scope: { type: 'global' }, canonical: '/home/test/.agents/mcp/global.json', config: { type: 'stdio', command: 'npx', args: ['-y', name] }, transport: 'stdio', description: null, targets: t('on', 'on', 'on'), overrides: {}, secretRefs: [], literalSecretKeys: [], isFavorite: false, tags: [], occurrences: [], ...extra });
					return [
						r('ai-memory', { description: 'Memória entre sessões', transport: 'http', config: { type: 'http', url: 'http://127.0.0.1:7700/mcp' }, isFavorite: true, occurrences: [{ uid: 'mcp:global:ai-memory', kind: 'mcp', name: 'ai-memory', cli: 'codex', scope: { type: 'global' }, filePath: '/home/test/.codex/config.toml' }] }),
						r('chrome-devtools', { description: 'Controle do Chrome', overrides: { codex: { args: [] }, agy: { args: [] } } }),
						r('icecat', { transport: 'http', description: 'Catálogo de produtos', secretRefs: ['ICECAT_API_TOKEN'], config: { type: 'http', url: 'https://icecat.example/mcp', headers: { 'Api-Token': '${ICECAT_API_TOKEN}' } } }),
						r('Neon', { transport: 'http', description: 'Postgres serverless', targets: t('providedByPlugin', 'providedByPlugin', 'on') }),
						r('events', { transport: 'sse', description: 'Eventos via SSE', targets: { claude: { state: 'on' }, codex: { state: 'unsupported', reason: 'Codex não fala SSE' }, agy: { state: 'on' } } }),
						r('supabase', { uid: 'mcp:project:/home/test/w/site:supabase', scope: { type: 'project', path: '/home/test/w/site' }, canonical: '/home/test/w/site/.mcp.json', transport: 'http', description: 'Banco do site', targets: t('on', 'on', 'off') }),
						r('brevo', { uid: 'mcp:project:/home/test/w/site:brevo', scope: { type: 'project', path: '/home/test/w/site' }, canonical: '/home/test/w/site/.mcp.json', transport: 'http', description: 'E-mail marketing', targets: t('off', 'off', 'off') }),
						{ uid: 'skill:global:frontend-design', kind: 'skill', name: 'frontend-design', scope: { type: 'global' }, canonical: '/home/test/.agents/skills/frontend-design', config: null, transport: null, description: 'Direção visual para UI nova', targets: t('on', 'native', 'native'), overrides: {}, secretRefs: [], literalSecretKeys: [], isFavorite: false, tags: [], occurrences: [] },
						{ uid: 'skill:global:debito-tecnico', kind: 'skill', name: 'debito-tecnico', scope: { type: 'global' }, canonical: '/home/test/.agents/skills/debito-tecnico', config: null, transport: null, description: 'Anotar débito técnico no TickTick', targets: t('off', 'native', 'native'), overrides: {}, secretRefs: [], literalSecretKeys: [], isFavorite: false, tags: [], occurrences: [] },
						{ uid: 'agent:global:reviewer', kind: 'agent', name: 'reviewer', scope: { type: 'global' }, canonical: '/home/test/.claude/agents/reviewer.md', config: null, transport: null, description: null, targets: { claude: { state: 'on' }, codex: { state: 'unsupported', reason: 'sem equivalente' }, agy: { state: 'unsupported', reason: 'sem equivalente' } }, overrides: {}, secretRefs: [], literalSecretKeys: [], isFavorite: false, tags: [], occurrences: [] }
					];
				}
				case 'get_project_sync':
					return {
						path: args?.project ?? '/home/test/w/site',
						name: 'site',
						managed: true,
						pending: ['agy'],
						servers: [
							{ name: 'supabase', inherited: false, overridesGlobal: false, transport: 'http', targets: { claude: { state: 'on' }, codex: { state: 'on' }, agy: { state: 'off' } }, secretRefs: [] },
							{ name: 'brevo', inherited: false, overridesGlobal: false, transport: 'http', targets: { claude: { state: 'off' }, codex: { state: 'off' }, agy: { state: 'off' } }, secretRefs: ['BREVO_MCP_TOKEN'] },
							{ name: 'ai-memory', inherited: true, overridesGlobal: false, transport: 'http', targets: { claude: { state: 'on' }, codex: { state: 'on' }, agy: { state: 'on' } }, secretRefs: [] },
							{ name: 'chrome-devtools', inherited: true, overridesGlobal: false, transport: 'stdio', targets: { claude: { state: 'on' }, codex: { state: 'on' }, agy: { state: 'on' } }, secretRefs: [] }
						],
						worktrees: [
							{ path: '/home/test/w/site', label: 'main', agyInSync: false },
							{ path: '/home/test/w/site-feature', label: 'site-feature', agyInSync: true }
						],
						report: []
					};
				case 'health_action_preview':
				case 'sync_preview': {
					const changes = [{ cli: 'agy', scope: { type: 'global' }, label: 'agy', path: '/home/test/.gemini/config/mcp_config.json', action: 'update', servers: ['comfyui'], diff: '--- a/home/test/.gemini/config/mcp_config.json\n+++ b/home/test/.gemini/config/mcp_config.json\n@@ -3,4 +3,4 @@\n     "comfyui": {\n-      "command": "old-node",\n+      "command": "node",\n       "env": { "TOKEN": "••••••" }\n' }];
					return cmd === 'sync_preview' ? { changes, report: [], inSync: false } : changes;
				}
				case 'sync_set_target':
				case 'sync_set_enabled':
				case 'sync_set_server':
				case 'sync_manage_project':
				case 'health_action_apply':
				case 'sync_apply':
				case 'set_skill_target':
					return { backupId: '20261124-120000-001', applied: ['/home/test/.codex/config.toml'], skipped: [], clis: ['codex'], unmanagedProject: false };
				case 'sync_undo':
					return [];
				case 'set_resource_favorite':
				case 'fix_secrets_permissions':
					return null;
				case 'get_cli_versions':
					return [];

				default:
					console.warn(`[Tauri Mock] Unhandled command: ${cmd}`);
					return null;
			}
		};

		// Define __TAURI_INTERNALS__ for Tauri v2
		Object.defineProperty(window, '__TAURI_INTERNALS__', {
			value: {
				invoke: mockInvoke,
				transformCallback: (callback: any) => callback,
				convertFileSrc: (src: string) => src
			},
			writable: true,
			configurable: true
		});

		// Also define __TAURI__ for compatibility
		Object.defineProperty(window, '__TAURI__', {
			value: {
				invoke: mockInvoke
			},
			writable: true,
			configurable: true
		});
	});
}

/**
 * Get the list of invoke calls made during the test
 */
export async function getInvokeCalls(page: Page): Promise<Array<{ cmd: string; args?: any; timestamp: number }>> {
	return await page.evaluate(() => (window as any).__INVOKE_CALLS__ || []);
}

/**
 * Clear the invoke call log
 */
export async function clearInvokeCalls(page: Page): Promise<void> {
	await page.evaluate(() => {
		(window as any).__INVOKE_CALLS__ = [];
	});
}
