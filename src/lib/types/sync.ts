// Types of the multi-CLI sync commands (Rust: src-tauri/src/sync, commands/sync.rs).

export type Cli = 'claude' | 'codex' | 'agy';
export const CLIS: Cli[] = ['claude', 'codex', 'agy'];

export type ResourceKind = 'mcp' | 'skill' | 'agent' | 'hook';

export type Scope = { type: 'global' } | { type: 'project'; path: string };

export type TargetState =
	| { state: 'on' }
	| { state: 'off' }
	| { state: 'providedByPlugin' }
	| { state: 'native' }
	| { state: 'unsupported'; reason: string };

export interface Occurrence {
	uid: string;
	kind: ResourceKind;
	name: string;
	cli: Cli;
	scope: Scope;
	filePath: string;
}

export interface ResourceView {
	uid: string;
	kind: ResourceKind;
	name: string;
	scope: Scope;
	canonical: string;
	config: Record<string, unknown> | null;
	transport: string | null;
	description: string | null;
	targets: Partial<Record<Cli, TargetState>>;
	overrides: Partial<Record<Cli, Record<string, unknown>>>;
	secretRefs: string[];
	literalSecretKeys: string[];
	isFavorite: boolean;
	tags: string[];
	occurrences: Occurrence[];
}

export type DivergenceKind = 'onlyInCli' | 'missingInCli' | 'drift' | 'claudeLocalScope' | 'agySkillsFolder';
export type HealthAction = 'import' | 'reapply' | 'removeLocalCopy' | 'refreshLocalCopy' | 'fixAgySkills';

export interface Divergence {
	kind: DivergenceKind;
	cli: Cli;
	scope: Scope;
	name: string;
	filePath: string;
	message: string;
	actions: HealthAction[];
}

export interface ReportItem {
	check: string;
	status: 'ok' | 'warn';
	summary: string;
	detail: string | null;
}

export interface CliVersion {
	cli: Cli;
	installed: string | null;
	tested: string;
	known: boolean;
	message: string | null;
}

export interface Health {
	inSync: boolean;
	resources: number;
	divergences: Divergence[];
	report: ReportItem[];
	checkedAt: string;
	missingSecrets: string[];
	secretsPermissionsOk: boolean;
	cliVersions: CliVersion[];
}

export interface ChangeView {
	cli: Cli;
	scope: Scope;
	label: string;
	path: string;
	action: 'create' | 'update' | 'delete';
	servers: string[];
	diff: string;
}

export interface PlanView {
	changes: ChangeView[];
	report: ReportItem[];
	inSync: boolean;
}

export interface ApplyOutcome {
	backupId: string | null;
	applied: string[];
	skipped: [string, string][];
}

export interface SaveOutcome extends ApplyOutcome {
	clis: Cli[];
	unmanagedProject: boolean;
}

export interface ProjectServer {
	name: string;
	inherited: boolean;
	overridesGlobal: boolean;
	transport: string;
	targets: Partial<Record<Cli, TargetState>>;
	secretRefs: string[];
}

export interface WorktreeStatus {
	path: string;
	label: string;
	agyInSync: boolean;
}

export interface ProjectSyncView {
	path: string;
	name: string;
	managed: boolean;
	pending: Cli[];
	servers: ProjectServer[];
	worktrees: WorktreeStatus[];
	report: ReportItem[];
}

export interface SecretsStatus {
	file: string;
	exists: boolean;
	permissionsOk: boolean;
	defined: string[];
	referenced: string[];
	missing: string[];
	unused: string[];
}

export interface McpManagementStatus {
	externallyManaged: boolean;
	agentsDir: string | null;
	globalSource: string | null;
	syncToolAvailable: boolean;
	message: string | null;
}

export const CLI_LABEL: Record<Cli, string> = {
	claude: 'Claude Code',
	codex: 'Codex',
	agy: 'Antigravity'
};

export const CLI_SHORT: Record<Cli, string> = {
	claude: 'Claude',
	codex: 'Codex',
	agy: 'agy'
};

export function scopeLabel(scope: Scope): string {
	if (scope.type === 'global') return 'global';
	const parts = scope.path.replace(/\\/g, '/').split('/').filter(Boolean);
	return parts[parts.length - 1] ?? scope.path;
}

/** Whether a target state can be toggled by the user. */
export function isToggleable(state: TargetState | undefined): boolean {
	return !!state && (state.state === 'on' || state.state === 'off');
}

/** Same as Rust `Scope::key`. */
export function scopeKey(scope: Scope): string {
	return scope.type === 'global' ? 'global' : `project:${scope.path.replace(/\\/g, '/').replace(/\/+$/, '')}`;
}

// What each CLI really loads (`get_live_mcp_status`).
export type LiveState = 'connected' | 'enabled' | 'disabled' | 'failed' | 'needsAuth';
export interface LiveServer {
	name: string;
	state: LiveState;
	detail: string;
}
export interface LiveCli {
	cli: Cli;
	servers: LiveServer[] | null;
	error: string | null;
	elapsedMs: number;
}
export interface TargetChange {
	project: string | null;
	name: string;
	cli: Cli;
	enabled: boolean;
}
