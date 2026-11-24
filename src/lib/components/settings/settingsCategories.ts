import { Sliders, ShieldCheck, Puzzle, Variable, ToggleRight, FileSearch, Clock, KeyRound, ServerCog, Keyboard, RotateCw, Building, Settings, Container, Bot, Terminal, Cloud, Users } from 'lucide-svelte';
import type { TranslationKey } from '$lib/i18n';

export type SettingsCategoryType = 'scoped' | 'standalone';

export interface SettingsCategory {
	id: string;
	label: string;
	labelKey: TranslationKey;
	icon: typeof Sliders;
	type: SettingsCategoryType;
}

export const SETTINGS_CATEGORIES: SettingsCategory[] = [
	{ id: 'models', label: 'Models', labelKey: 'settings.cat.models', icon: Sliders, type: 'scoped' },
	{ id: 'security', label: 'Security', labelKey: 'settings.cat.security', icon: ShieldCheck, type: 'scoped' },
	{ id: 'plugins', label: 'Plugins', labelKey: 'settings.cat.plugins', icon: Puzzle, type: 'scoped' },
	{ id: 'environment', label: 'Environment', labelKey: 'settings.cat.environment', icon: Variable, type: 'scoped' },
	{ id: 'interface', label: 'Interface', labelKey: 'settings.cat.interface', icon: ToggleRight, type: 'scoped' },
	{ id: 'files', label: 'Files', labelKey: 'settings.cat.files', icon: FileSearch, type: 'scoped' },
	{ id: 'session', label: 'Session', labelKey: 'settings.cat.session', icon: Clock, type: 'scoped' },
	{ id: 'auto-mode', label: 'Auto Mode', labelKey: 'settings.cat.autoMode', icon: Bot, type: 'scoped' },
	{ id: 'authentication', label: 'Auth', labelKey: 'settings.cat.authentication', icon: KeyRound, type: 'scoped' },
	{ id: 'agent-teams', label: 'Agent Teams', labelKey: 'settings.cat.agentTeams', icon: Users, type: 'scoped' },
	{ id: 'mcp-approval', label: 'MCP Approval', labelKey: 'settings.cat.mcpApproval', icon: ServerCog, type: 'scoped' },
	{ id: 'keybindings', label: 'Keybindings', labelKey: 'settings.cat.keybindings', icon: Keyboard, type: 'standalone' },
	{ id: 'spinner-verbs', label: 'Spinner Verbs', labelKey: 'settings.cat.spinnerVerbs', icon: RotateCw, type: 'standalone' },
	{ id: 'admin', label: 'Admin', labelKey: 'settings.cat.admin', icon: Building, type: 'standalone' },
	{ id: 'cli', label: 'CLI & Scheduling', labelKey: 'settings.cat.cli', icon: Terminal, type: 'standalone' },
	{ id: 'cloud-sync', label: 'Cloud Sync', labelKey: 'settings.cat.cloudSync', icon: Cloud, type: 'standalone' },
	{ id: 'editor-sync', label: 'Editor Sync', labelKey: 'settings.cat.editorSync', icon: Settings, type: 'standalone' },
	{ id: 'containers', label: 'Containers', labelKey: 'settings.cat.containers', icon: Container, type: 'standalone' }
];
