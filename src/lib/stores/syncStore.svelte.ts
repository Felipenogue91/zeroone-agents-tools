import { invoke } from '@tauri-apps/api/core';
import { i18n } from '$lib/i18n';
import { notifications } from './notifications.svelte';
import type {
	ChangeView,
	Cli,
	Divergence,
	Health,
	HealthAction,
	McpManagementStatus,
	PlanView,
	ProjectSyncView,
	ResourceView,
	SaveOutcome,
	ApplyOutcome,
	LiveCli,
	TargetChange
} from '$lib/types/sync';
import { CLI_SHORT } from '$lib/types/sync';

function projectArg(scopePath: string | null | undefined) {
	return scopePath ?? null;
}

/**
 * Multi-CLI sync state. `~/.agents` is the source of truth; this store only
 * mirrors what the backend reports and refreshes when the config watcher
 * emits `sync-changed`.
 */
class SyncState {
	health = $state<Health | null>(null);
	resources = $state<ResourceView[]>([]);
	management = $state<McpManagementStatus | null>(null);
	isLoadingHealth = $state(false);
	isLoadingResources = $state(false);
	error = $state<string | null>(null);
	/** What each CLI really loads (global view), asked to the CLIs. */
	live = $state<Partial<Record<Cli, LiveCli>> | null>(null);
	isLoadingLive = $state(false);
	/** uid/name currently being saved (disables its controls). */
	busy = $state<Set<string>>(new Set());

	private listening = false;
	private refreshTimer: ReturnType<typeof setTimeout> | null = null;

	get divergenceCount(): number {
		return this.health?.divergences.length ?? 0;
	}

	async loadHealth() {
		this.isLoadingHealth = true;
		this.error = null;
		try {
			this.health = await invoke<Health>('get_sync_health');
		} catch (e) {
			this.error = String(e);
		} finally {
			this.isLoadingHealth = false;
		}
	}

	async loadResources() {
		this.isLoadingResources = true;
		try {
			this.resources = await invoke<ResourceView[]>('get_sync_resources');
		} catch (e) {
			this.error = String(e);
		} finally {
			this.isLoadingResources = false;
		}
	}

	async loadLive() {
		this.isLoadingLive = true;
		try {
			this.live = await invoke<Partial<Record<Cli, LiveCli>>>('get_live_mcp_status', { project: null });
		} catch (e) {
			console.error('[sync] live status failed', e);
		} finally {
			this.isLoadingLive = false;
		}
	}

	/** Several switches as one undoable set. */
	setTargets(changes: TargetChange[]) {
		return this.save('bulk', () => invoke<SaveOutcome>('sync_set_targets', { changes }));
	}

	async loadManagement() {
		try {
			this.management = await invoke<McpManagementStatus>('get_mcp_management_status');
		} catch {
			this.management = null;
		}
	}

	async refresh() {
		await Promise.all([this.loadHealth(), this.loadResources()]);
	}

	/** Subscribe once to the backend watcher (debounced refresh). */
	async listen() {
		if (this.listening) return;
		this.listening = true;
		try {
			const { listen } = await import('@tauri-apps/api/event');
			await listen<string[]>('sync-changed', () => {
				if (this.refreshTimer) clearTimeout(this.refreshTimer);
				this.refreshTimer = setTimeout(() => this.refresh(), 300);
			});
		} catch {
			// not running inside Tauri (tests / browser preview)
		}
	}

	private mark(key: string, on: boolean) {
		const next = new Set(this.busy);
		if (on) next.add(key);
		else next.delete(key);
		this.busy = next;
	}

	isBusy(key: string) {
		return this.busy.has(key);
	}

	async undo(backupId: string) {
		try {
			await invoke<string[]>('sync_undo', { backupId });
			notifications.info(i18n.t('sync.undone'));
			await this.refresh();
		} catch (e) {
			notifications.error(i18n.t('sync.undoFailed'), 8000);
			console.error('[sync] undo failed', e);
		}
	}

	/** "Salvo em N CLIs · Desfazer" toast for any write. */
	notifySaved(out: ApplyOutcome & { clis?: Cli[]; unmanagedProject?: boolean }) {
		const skipped = out.skipped?.length ?? 0;
		if (!out.backupId && !out.applied.length) {
			if (skipped) {
				notifications.add('warning', i18n.t('sync.skippedChanged'), { duration: 8000, detail: out.skipped.map((s) => s[0]).join('\n') });
			} else {
				notifications.info(i18n.t('sync.nothingToDo'));
			}
			return;
		}
		const clis = out.clis ?? [];
		const msg = clis.length
			? i18n.t('sync.savedIn', { n: clis.length, clis: clis.map((c) => CLI_SHORT[c]).join(', ') })
			: i18n.t('sync.saved');
		const backupId = out.backupId;
		notifications.add(skipped ? 'warning' : 'success', msg, {
			duration: 8000,
			detail: out.unmanagedProject ? i18n.t('sync.unmanagedHint') : skipped ? i18n.t('sync.skippedChanged') : undefined,
			action: backupId ? { label: i18n.t('sync.undo'), onclick: () => this.undo(backupId) } : undefined
		});
	}

	private async save(key: string, run: () => Promise<SaveOutcome | ApplyOutcome>) {
		this.mark(key, true);
		try {
			const out = await run();
			this.notifySaved(out);
			await this.refresh();
			return out;
		} catch (e) {
			notifications.add('error', i18n.t('sync.saveFailed'), { duration: 10000, detail: String(e) });
			console.error('[sync] save failed', e);
			return null;
		} finally {
			this.mark(key, false);
		}
	}

	setTarget(name: string, cli: Cli, enabled: boolean, project?: string | null) {
		return this.save(`${project ?? 'global'}:${name}`, () =>
			invoke<SaveOutcome>('sync_set_target', { project: projectArg(project), name, cli, enabled })
		);
	}

	setEnabled(project: string, name: string, enabled: boolean) {
		return this.save(`${project}:${name}`, () => invoke<SaveOutcome>('sync_set_enabled', { project, name, enabled }));
	}

	setServer(name: string, config: Record<string, unknown> | null, project?: string | null) {
		return this.save(`${project ?? 'global'}:${name}`, () =>
			invoke<SaveOutcome>('sync_set_server', { project: projectArg(project), name, config })
		);
	}

	manageProject(project: string, managed: boolean) {
		return this.save(`manage:${project}`, () => invoke<SaveOutcome>('sync_manage_project', { project, managed }));
	}

	setSkillTarget(name: string, cli: Cli, enabled: boolean) {
		return this.save(`skill:${name}`, async () => {
			const out = await invoke<ApplyOutcome | null>('set_skill_target', { name, cli, enabled });
			return out ?? { backupId: null, applied: [`skill:${name}`], skipped: [], clis: [cli] };
		});
	}

	private divergenceArgs(d: Divergence, action: HealthAction) {
		return {
			action,
			cli: d.cli,
			project: d.scope.type === 'project' ? d.scope.path : null,
			name: d.name,
			filePath: d.filePath
		};
	}

	previewAction(d: Divergence, action: HealthAction) {
		return invoke<ChangeView[]>('health_action_preview', this.divergenceArgs(d, action));
	}

	applyAction(d: Divergence, action: HealthAction) {
		return this.save(`div:${d.cli}:${d.name}:${action}`, () =>
			invoke<ApplyOutcome>('health_action_apply', this.divergenceArgs(d, action))
		);
	}

	previewAll(project?: string | null) {
		return invoke<PlanView>('sync_preview', { project: projectArg(project) });
	}

	applyAll(project?: string | null) {
		return this.save(`all:${project ?? 'global'}`, () => invoke<ApplyOutcome>('sync_apply', { project: projectArg(project) }));
	}

	loadProject(path: string) {
		return invoke<ProjectSyncView>('get_project_sync', { project: path });
	}
}

export const syncStore = new SyncState();
