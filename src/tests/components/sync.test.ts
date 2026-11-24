import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import { invoke } from '@tauri-apps/api/core';
import { detectLocale } from '$lib/i18n/i18n.svelte';
import { scopeKey, scopeLabel, isToggleable } from '$lib/types/sync';
import type { ResourceView } from '$lib/types/sync';

vi.mock('$app/navigation', () => ({ goto: vi.fn() }));

const invokeMock = invoke as unknown as ReturnType<typeof vi.fn>;

function resource(partial: Partial<ResourceView>): ResourceView {
	return {
		uid: `mcp:global:${partial.name ?? 'x'}`,
		kind: 'mcp',
		name: 'x',
		scope: { type: 'global' },
		canonical: '~/.agents/mcp/global.json',
		config: {},
		transport: 'stdio',
		description: null,
		targets: { claude: { state: 'on' }, codex: { state: 'on' }, agy: { state: 'on' } },
		overrides: {},
		secretRefs: [],
		literalSecretKeys: [],
		isFavorite: false,
		tags: [],
		occurrences: [],
		...partial
	};
}

describe('locale detection', () => {
	it('defaults to pt-BR even when the webview reports en-US', () => {
		expect(detectLocale(['en-US', 'en'])).toBe('pt-BR');
		expect(detectLocale([])).toBe('pt-BR');
		expect(detectLocale(['pt-BR'])).toBe('pt-BR');
	});

	it('keeps Chinese system languages', () => {
		expect(detectLocale(['zh-TW'])).toBe('zh-TW');
		expect(detectLocale(['zh-Hant-HK'])).toBe('zh-TW');
		expect(detectLocale(['zh-CN'])).toBe('zh-CN');
	});
});

describe('sync type helpers', () => {
	it('matches the Rust scope key', () => {
		expect(scopeKey({ type: 'global' })).toBe('global');
		expect(scopeKey({ type: 'project', path: 'C:\\w\\proj\\' })).toBe('project:C:/w/proj');
		expect(scopeLabel({ type: 'project', path: '/home/u/w/site/' })).toBe('site');
	});

	it('only on/off states are toggleable', () => {
		expect(isToggleable({ state: 'on' })).toBe(true);
		expect(isToggleable({ state: 'off' })).toBe(true);
		expect(isToggleable({ state: 'native' })).toBe(false);
		expect(isToggleable({ state: 'providedByPlugin' })).toBe(false);
		expect(isToggleable({ state: 'unsupported', reason: 'x' })).toBe(false);
		expect(isToggleable(undefined)).toBe(false);
	});
});

describe('CliChip', () => {
	it('shows the state with text and aria-pressed, not only color', async () => {
		const { default: CliChip } = await import('$lib/components/sync/CliChip.svelte');
		const onToggle = vi.fn();
		render(CliChip, { props: { cli: 'codex', state: { state: 'on' }, onToggle } });
		const btn = screen.getByRole('button');
		expect(btn).toHaveAttribute('aria-pressed', 'true');
		expect(btn.textContent).toContain('Codex');
		await fireEvent.click(btn);
		expect(onToggle).toHaveBeenCalledWith(false);
	});

	it('is not clickable when the CLI reads the source natively', async () => {
		const { default: CliChip } = await import('$lib/components/sync/CliChip.svelte');
		const onToggle = vi.fn();
		render(CliChip, { props: { cli: 'codex', state: { state: 'native' }, onToggle } });
		const btn = screen.getByRole('button');
		expect(btn).toBeDisabled();
		await fireEvent.click(btn);
		expect(onToggle).not.toHaveBeenCalled();
	});

	it('explains unsupported states with the backend reason', async () => {
		const { default: CliChip } = await import('$lib/components/sync/CliChip.svelte');
		render(CliChip, { props: { cli: 'codex', state: { state: 'unsupported', reason: 'Codex não fala SSE' } } });
		expect(screen.getByTitle('Codex não fala SSE')).toBeInTheDocument();
	});
});

describe('DiffView', () => {
	it('renders added/removed lines and the masking note', async () => {
		const { default: DiffView } = await import('$lib/components/sync/DiffView.svelte');
		render(DiffView, { props: { diff: '--- a\n+++ b\n@@ -1 +1 @@\n-old\n+new\n' } });
		expect(screen.getByText('+new')).toBeInTheDocument();
		expect(screen.getByText('-old')).toBeInTheDocument();
		expect(screen.getByText(/hidden/)).toBeInTheDocument();
	});

	it('says when there is nothing to change', async () => {
		const { default: DiffView } = await import('$lib/components/sync/DiffView.svelte');
		render(DiffView, { props: { diff: '' } });
		expect(screen.getByText('No changes')).toBeInTheDocument();
	});
});

describe('ResourceList', () => {
	const list = [
		resource({ name: 'ai-memory', description: 'memory' }),
		resource({ name: 'supabase', uid: 'mcp:project:/w/site:supabase', scope: { type: 'project', path: '/w/site' } }),
		resource({ name: 'shared-skill', uid: 'skill:global:shared-skill', kind: 'skill', targets: { claude: { state: 'off' }, codex: { state: 'native' }, agy: { state: 'native' } } })
	];

	it('filters by kind, origin, CLI and text', async () => {
		const { default: ResourceList } = await import('$lib/components/sync/ResourceList.svelte');
		render(ResourceList, { props: { resources: list, onOpen: vi.fn() } });
		expect(screen.getAllByRole('button', { name: /\(global\)|\(site\)/ }).length).toBe(3);

		await fireEvent.click(screen.getByRole('tab', { name: /Skills/ }));
		expect(screen.getByRole('button', { name: 'shared-skill (global)' })).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'ai-memory (global)' })).not.toBeInTheDocument();

		await fireEvent.click(screen.getByRole('tab', { name: /All/ }));
		await fireEvent.input(screen.getByPlaceholderText('Search by name or description'), { target: { value: 'supa' } });
		expect(screen.getByRole('button', { name: 'supabase (site)' })).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'ai-memory (global)' })).not.toBeInTheDocument();
	});

	it('opens the detail on Enter and shows an empty state with an action', async () => {
		const { default: ResourceList } = await import('$lib/components/sync/ResourceList.svelte');
		const onOpen = vi.fn();
		const onAdd = vi.fn();
		const { unmount } = render(ResourceList, { props: { resources: list, onOpen } });
		await fireEvent.keyDown(screen.getByRole('button', { name: 'ai-memory (global)' }), { key: 'Enter' });
		expect(onOpen).toHaveBeenCalledWith(expect.objectContaining({ name: 'ai-memory' }));
		unmount();
		render(ResourceList, { props: { resources: [], onOpen, onAdd } });
		expect(screen.getByText('Nothing here yet')).toBeInTheDocument();
		await fireEvent.click(screen.getAllByRole('button', { name: 'Add MCP' })[1]);
		expect(onAdd).toHaveBeenCalled();
	});

	it('toggling a chip calls the per-CLI command for that scope', async () => {
		invokeMock.mockReset();
		invokeMock.mockResolvedValue({ backupId: 'b1', applied: ['x'], skipped: [], clis: ['codex'], unmanagedProject: false });
		const { default: ResourceList } = await import('$lib/components/sync/ResourceList.svelte');
		render(ResourceList, { props: { resources: [list[1]], onOpen: vi.fn() } });
		await fireEvent.click(screen.getByRole('button', { name: /^Codex:/ }));
		expect(invokeMock).toHaveBeenCalledWith('sync_set_target', { project: '/w/site', name: 'supabase', cli: 'codex', enabled: false });
	});
});

describe('syncStore', () => {
	beforeEach(() => invokeMock.mockReset());

	it('save toast offers undo with the backup id', async () => {
		const { syncStore } = await import('$lib/stores/syncStore.svelte');
		const { notifications } = await import('$lib/stores/notifications.svelte');
		notifications.clear();
		syncStore.notifySaved({ backupId: 'bk-1', applied: ['/a'], skipped: [], clis: ['claude', 'agy'] });
		const n = notifications.notifications.at(-1)!;
		expect(n.message).toContain('2');
		expect(n.action?.label).toBe('Undo');
		invokeMock.mockResolvedValue([]);
		n.action!.onclick();
		expect(invokeMock).toHaveBeenCalledWith('sync_undo', { backupId: 'bk-1' });
	});

	it('reports when nothing changed', async () => {
		const { syncStore } = await import('$lib/stores/syncStore.svelte');
		const { notifications } = await import('$lib/stores/notifications.svelte');
		notifications.clear();
		syncStore.notifySaved({ backupId: null, applied: [], skipped: [] });
		expect(notifications.notifications.at(-1)?.message).toBe('Nothing to change');
	});
});

describe('CommandPalette', () => {
	it('opens with Ctrl+K and finds pages', async () => {
		invokeMock.mockResolvedValue([]);
		const { default: CommandPalette } = await import('$lib/components/sync/CommandPalette.svelte');
		render(CommandPalette);
		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
		await fireEvent.keyDown(window, { key: 'k', ctrlKey: true });
		expect(screen.getByRole('dialog')).toBeInTheDocument();
		await fireEvent.input(screen.getByRole('combobox'), { target: { value: 'clis' } });
		expect(screen.getByRole('option', { name: /CLIs/ })).toBeInTheDocument();
		await fireEvent.keyDown(window, { key: 'Escape' });
		expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
	});
});
