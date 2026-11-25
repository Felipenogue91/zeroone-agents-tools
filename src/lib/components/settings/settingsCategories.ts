import { Sliders, ShieldCheck, Puzzle, Variable, ToggleRight, FileSearch, Clock, KeyRound, ServerCog, Keyboard, RotateCw, Building, Settings, Container, Bot, Terminal, Cloud, Users } from 'lucide-svelte';
import type { TranslationKey } from '$lib/i18n';

export type SettingsCategoryType = 'scoped' | 'standalone';

export interface SettingsCategory {
	id: string;
	label: string;
	labelKey: TranslationKey;
	icon: typeof Sliders;
	type: SettingsCategoryType;
	/** What the section configures and what pain it solves (hover). */
	tipKey: TranslationKey;
}

export const SETTINGS_CATEGORIES: SettingsCategory[] = [
	{ id: 'models', label: 'Models', labelKey: 'settings.cat.models', icon: Sliders, type: 'scoped', tipKey: 'tip.settingsCat.models' },
	{ id: 'security', label: 'Security', labelKey: 'settings.cat.security', icon: ShieldCheck, type: 'scoped', tipKey: 'tip.settingsCat.security' },
	{ id: 'plugins', label: 'Plugins', labelKey: 'settings.cat.plugins', icon: Puzzle, type: 'scoped', tipKey: 'tip.settingsCat.plugins' },
	{ id: 'environment', label: 'Environment', labelKey: 'settings.cat.environment', icon: Variable, type: 'scoped', tipKey: 'tip.settingsCat.environment' },
	{ id: 'interface', label: 'Interface', labelKey: 'settings.cat.interface', icon: ToggleRight, type: 'scoped', tipKey: 'tip.settingsCat.interface' },
	{ id: 'files', label: 'Files', labelKey: 'settings.cat.files', icon: FileSearch, type: 'scoped', tipKey: 'tip.settingsCat.files' },
	{ id: 'session', label: 'Session', labelKey: 'settings.cat.session', icon: Clock, type: 'scoped', tipKey: 'tip.settingsCat.session' },
	{ id: 'auto-mode', label: 'Auto Mode', labelKey: 'settings.cat.autoMode', icon: Bot, type: 'scoped', tipKey: 'tip.settingsCat.auto_mode' },
	{ id: 'authentication', label: 'Auth', labelKey: 'settings.cat.authentication', icon: KeyRound, type: 'scoped', tipKey: 'tip.settingsCat.authentication' },
	{ id: 'agent-teams', label: 'Agent Teams', labelKey: 'settings.cat.agentTeams', icon: Users, type: 'scoped', tipKey: 'tip.settingsCat.agent_teams' },
	{ id: 'mcp-approval', label: 'MCP Approval', labelKey: 'settings.cat.mcpApproval', icon: ServerCog, type: 'scoped', tipKey: 'tip.settingsCat.mcp_approval' },
	{ id: 'keybindings', label: 'Keybindings', labelKey: 'settings.cat.keybindings', icon: Keyboard, type: 'standalone', tipKey: 'tip.settingsCat.keybindings' },
	{ id: 'spinner-verbs', label: 'Spinner Verbs', labelKey: 'settings.cat.spinnerVerbs', icon: RotateCw, type: 'standalone', tipKey: 'tip.settingsCat.spinner_verbs' },
	{ id: 'admin', label: 'Admin', labelKey: 'settings.cat.admin', icon: Building, type: 'standalone', tipKey: 'tip.settingsCat.admin' },
	{ id: 'cli', label: 'CLI & Scheduling', labelKey: 'settings.cat.cli', icon: Terminal, type: 'standalone', tipKey: 'tip.settingsCat.cli' },
	{ id: 'cloud-sync', label: 'Cloud Sync', labelKey: 'settings.cat.cloudSync', icon: Cloud, type: 'standalone', tipKey: 'tip.settingsCat.cloud_sync' },
	{ id: 'editor-sync', label: 'Editor Sync', labelKey: 'settings.cat.editorSync', icon: Settings, type: 'standalone', tipKey: 'tip.settingsCat.editor_sync' },
	{ id: 'containers', label: 'Containers', labelKey: 'settings.cat.containers', icon: Container, type: 'standalone', tipKey: 'tip.settingsCat.containers' }
];
