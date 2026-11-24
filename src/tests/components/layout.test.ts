import { describe, it, expect, vi, beforeAll } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';

vi.mock('$lib/stores', () => ({
	mcpLibrary: { load: vi.fn(), mcps: [] },
	projectsStore: { loadProjects: vi.fn(), loadGlobalMcps: vi.fn(), projects: [] },
	notifications: { success: vi.fn(), error: vi.fn() },
	sessionStore: {
		projects: [],
		isLoading: false,
		isLoadingProjects: false,
		load: vi.fn(),
		loadProjects: vi.fn()
	},
	i18n: {
		t: (key: string) => {
			const translations: Record<string, string> = {
				'app.name': 'ZeroOne',
				'app.tagline': 'Agents Tools',
				'nav.core': 'Core',
				'nav.tools': 'Tools',
				'nav.configure': 'Configure',
				'nav.insights': 'Insights',
				'nav.dashboard': 'Dashboard',
				'nav.projects': 'Projects',
				'nav.mcps': 'MCPs',
				'nav.agents': 'Agents',
				'nav.skills': 'Skills',
				'nav.commands': 'Commands',
				'nav.hooks': 'Hooks',
				'nav.rules': 'Rules',
				'nav.containers': 'Containers',
				'nav.profiles': 'Profiles',
				'nav.statusLine': 'Status Line',
				'nav.permissions': 'Permissions',
				'nav.memory': 'Memory',
				'nav.marketplace': 'Marketplace',
				'nav.analytics': 'Analytics',
				'nav.comparison': 'Comparison',
				'nav.sessions': 'Sessions',
				'nav.settings': 'Settings',
				'nav.home': 'Home',
				'nav.library': 'Library',
				'nav.clis': 'CLIs',
				'nav.advanced': 'Claude Code advanced',
				'header.search': 'Search…',
				'header.searchTitle': 'Search pages, MCPs, skills and projects (Ctrl+K)',
				'sidebar.collapse': 'Collapse',
				'sidebar.collapseTitle': 'Collapse sidebar',
				'sidebar.expandTitle': 'Expand sidebar',
				'sidebar.navLabel': 'Main navigation',
				'header.refresh': 'Refresh',
				'header.toggleTheme': 'Toggle theme',
			};
			return translations[key] ?? key;
		},
		setLocale: vi.fn()
	}
}));

vi.mock('$app/stores', () => ({
	page: {
		subscribe: vi.fn((cb: any) => {
			cb({ url: new URL('http://localhost/') });
			return () => {};
		})
	}
}));

vi.mock('@tauri-apps/api/app', () => ({
	getVersion: vi.fn().mockResolvedValue('3.2.4')
}));

vi.mock('$lib/types/usage', () => ({
	estimateSessionCost: vi.fn().mockReturnValue(0),
	formatCost: vi.fn().mockReturnValue('$0.00'),
	formatCompactNumber: vi.fn().mockReturnValue('0')
}));

describe('Header Component', () => {
	let Header: any;

	beforeAll(async () => {
		const mod = await import('$lib/components/layout/Header.svelte');
		Header = mod.default;
	});

	it('should render title', () => {
		render(Header, { props: { title: 'Test Page' } });
		expect(screen.getByText('Test Page')).toBeInTheDocument();
	});

	it('should render subtitle when provided', () => {
		render(Header, { props: { title: 'Page', subtitle: 'Sub info' } });
		expect(screen.getByText('Sub info')).toBeInTheDocument();
	});

	it('should not render subtitle when not provided', () => {
		render(Header, { props: { title: 'Page' } });
		expect(screen.queryByText('Sub info')).not.toBeInTheDocument();
	});

	it('should have refresh button', () => {
		render(Header, { props: { title: 'Page' } });
		expect(screen.getByLabelText('Refresh')).toBeInTheDocument();
	});

	it('should have theme toggle button', () => {
		render(Header, { props: { title: 'Page' } });
		const themeBtn = screen.getByLabelText('Toggle theme');
		expect(themeBtn).toBeInTheDocument();
	});

	it('should have a search button that opens the command palette', async () => {
		render(Header, { props: { title: 'Page' } });
		const opened = vi.fn();
		window.addEventListener('open-command-palette', opened);
		await fireEvent.click(screen.getByLabelText('Search pages, MCPs, skills and projects (Ctrl+K)'));
		expect(opened).toHaveBeenCalled();
		window.removeEventListener('open-command-palette', opened);
	});
});

describe('TodayUsageWidget Component', () => {
	let TodayUsageWidget: any;

	beforeAll(async () => {
		const mod = await import('$lib/components/layout/TodayUsageWidget.svelte');
		TodayUsageWidget = mod.default;
	});

	it('should render without data (empty)', () => {
		render(TodayUsageWidget);
		// When no projects, widget should not show content
		expect(screen.queryByText('Usage')).not.toBeInTheDocument();
	});

	it('should render usage widget when projects exist', async () => {
		const { sessionStore } = await import('$lib/stores');
		(sessionStore as any).projects = [
			{
				folderName: 'proj',
				inferredPath: '/proj',
				sessionCount: 5,
				totalInputTokens: 1000,
				totalOutputTokens: 500,
				totalCacheReadTokens: 0,
				totalCacheCreationTokens: 0,
				modelsUsed: ['sonnet'],
				toolUsage: {},
				latestSession: null,
				earliestSession: null
			}
		];
		render(TodayUsageWidget);
		expect(screen.getByText('Usage')).toBeInTheDocument();
		expect(screen.getByText('Projects')).toBeInTheDocument();
		expect(screen.getByText('Sessions')).toBeInTheDocument();
		expect(screen.getByText('Cost')).toBeInTheDocument();
		(sessionStore as any).projects = [];
	});
});

describe('Sidebar Component', () => {
	let Sidebar: any;

	beforeAll(async () => {
		const mod = await import('$lib/components/layout/Sidebar.svelte');
		Sidebar = mod.default;
	});

	it('should render ZeroOne heading', () => {
		render(Sidebar);
		expect(screen.getByText('ZeroOne')).toBeInTheDocument();
	});

	it('should render Agents Tools subtitle', () => {
		render(Sidebar);
		expect(screen.getByText('Agents Tools')).toBeInTheDocument();
	});

	it('should render the task-based main navigation', () => {
		render(Sidebar);
		expect(screen.getByText('Home')).toBeInTheDocument();
		expect(screen.getByText('Projects')).toBeInTheDocument();
		expect(screen.getByText('Library')).toBeInTheDocument();
		expect(screen.getByText('CLIs')).toBeInTheDocument();
	});

	it('should keep Claude Code advanced pages in a collapsible group', async () => {
		try {
			localStorage.removeItem('nav.advancedOpen');
		} catch {
			// ignore
		}
		render(Sidebar);
		const toggle = screen.getByText('Claude Code advanced');
		expect(screen.queryByText('Analytics')).not.toBeInTheDocument();
		await fireEvent.click(toggle);
		expect(screen.getByText('Profiles')).toBeInTheDocument();
		expect(screen.getByText('Permissions')).toBeInTheDocument();
		expect(screen.getByText('Memory')).toBeInTheDocument();
		expect(screen.getByText('Analytics')).toBeInTheDocument();
		expect(screen.getByText('Sessions')).toBeInTheDocument();
		expect(screen.getByText('Containers')).toBeInTheDocument();
	});

	it('should render Settings link', () => {
		render(Sidebar);
		expect(screen.getByText('Settings')).toBeInTheDocument();
	});

	it('should render Collapse button', () => {
		render(Sidebar);
		expect(screen.getByText('Collapse')).toBeInTheDocument();
	});
});

describe('Layout index.ts exports', () => {
	let layoutExports: any;

	beforeAll(async () => {
		layoutExports = await import('$lib/components/layout');
	});

	it('should export all layout components', () => {
		expect(layoutExports.Sidebar).toBeDefined();
		expect(layoutExports.Header).toBeDefined();
	});
});
