<script lang="ts">
	import { page } from '$app/stores';
	import { House, FolderOpen, Library, TerminalSquare, Settings, ChevronRight, Container, PanelBottom, BarChart3, TrendingUp, FolderSearch, GitCompareArrows, Layers, FileText, Shield, Store, PanelLeftClose, PanelLeftOpen, Plug } from 'lucide-svelte';
	import { onMount, onDestroy } from 'svelte';
	import { getVersion } from '@tauri-apps/api/app';
	import TodayUsageWidget from './TodayUsageWidget.svelte';
	import { sessionStore, i18n } from '$lib/stores';
	import { syncStore } from '$lib/stores/syncStore.svelte';

	let appVersion = $state('');
	let collapsed = $state(false);
	let manualOverride = $state(false);
	let windowWidth = $state(window.innerWidth);
	let advancedOpen = $state(readAdvanced());

	const AUTO_COLLAPSE_WIDTH = 860;
	let resizeRaf: number | null = null;

	function readAdvanced(): boolean {
		try {
			return localStorage.getItem('nav.advancedOpen') === '1';
		} catch {
			return false;
		}
	}

	function toggleAdvanced() {
		advancedOpen = !advancedOpen;
		try {
			localStorage.setItem('nav.advancedOpen', advancedOpen ? '1' : '0');
		} catch {
			// storage unavailable
		}
	}

	function handleResize() {
		if (resizeRaf) return;
		resizeRaf = requestAnimationFrame(() => {
			resizeRaf = null;
			windowWidth = window.innerWidth;
			if (!manualOverride) collapsed = windowWidth < AUTO_COLLAPSE_WIDTH;
		});
	}

	function toggleCollapse() {
		manualOverride = true;
		collapsed = !collapsed;
	}

	onMount(async () => {
		window.addEventListener('resize', handleResize);
		handleResize();
		try {
			appVersion = await getVersion();
		} catch {
			appVersion = '1.0.0';
		}
		if (sessionStore.projects.length === 0) sessionStore.loadProjects();
	});

	onDestroy(() => {
		window.removeEventListener('resize', handleResize);
		if (resizeRaf) cancelAnimationFrame(resizeRaf);
	});

	interface NavItem {
		href: string;
		label: string;
		icon: typeof Plug;
		/** Extra paths that also mark the item as active. */
		match?: string[];
		badge?: number;
	}

	const libraryPaths = ['/library', '/skills', '/subagents', '/hooks', '/commands', '/rules'];

	let mainItems: NavItem[] = $derived([
		{ href: '/', label: i18n.t('nav.home'), icon: House, badge: syncStore.divergenceCount },
		{ href: '/projects', label: i18n.t('nav.projects'), icon: FolderOpen },
		{ href: '/library', label: i18n.t('nav.library'), icon: Library, match: libraryPaths },
		{ href: '/clis', label: i18n.t('nav.clis'), icon: TerminalSquare }
	]);

	let advancedItems: NavItem[] = $derived([
		{ href: '/profiles', label: i18n.t('nav.profiles'), icon: Layers },
		{ href: '/permissions', label: i18n.t('nav.permissions'), icon: Shield },
		{ href: '/memory', label: i18n.t('nav.memory'), icon: FileText },
		{ href: '/statusline', label: i18n.t('nav.statusLine'), icon: PanelBottom },
		{ href: '/analytics', label: i18n.t('nav.analytics'), icon: BarChart3 },
		{ href: '/insights', label: i18n.t('nav.insights'), icon: TrendingUp },
		{ href: '/sessions', label: i18n.t('nav.sessions'), icon: FolderSearch },
		{ href: '/comparison', label: i18n.t('nav.comparison'), icon: GitCompareArrows },
		{ href: '/containers', label: i18n.t('nav.containers'), icon: Container },
		{ href: '/marketplace', label: i18n.t('nav.marketplace'), icon: Store }
	]);

	function isActive(item: NavItem): boolean {
		const p = $page.url.pathname;
		if (item.href === '/') return p === '/';
		const paths = item.match ?? [item.href];
		return paths.some((h) => p === h || p.startsWith(h + '/'));
	}

	let advancedActive = $derived(advancedItems.some((i) => isActive(i)));
	let isSettingsActive = $derived($page.url.pathname.startsWith('/settings'));
</script>

<aside class="border-r border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 flex flex-col transition-[width] duration-150 {collapsed ? 'w-16' : 'w-56'}">
	<div class="p-4 border-b border-gray-200 dark:border-gray-700 flex items-center {collapsed ? 'justify-center' : 'gap-3'}">
		<img src="/brand/zeroone-icone-app.svg" alt="" aria-hidden="true" class="w-9 h-9 shrink-0 select-none" draggable="false" />
		{#if !collapsed}
			<div class="overflow-hidden">
				<h1 class="font-bold text-base tracking-tight text-gray-900 dark:text-white whitespace-nowrap leading-tight">{i18n.t('app.name')}</h1>
				<p class="text-xs font-medium text-primary-600 dark:text-primary-400 whitespace-nowrap">{i18n.t('app.tagline')}</p>
			</div>
		{/if}
	</div>

	<nav aria-label={i18n.t('sidebar.navLabel')} class="flex-1 overflow-y-auto p-3 {collapsed ? 'px-2' : ''}">
		<div class="space-y-0.5">
			{#each mainItems as item (item.href)}
				{@render navLink(item)}
			{/each}
		</div>

		<div class="mt-4">
			{#if collapsed}
				<div class="h-px bg-gray-200 dark:bg-gray-700 mx-2 mb-2"></div>
			{:else}
				<button onclick={toggleAdvanced} aria-expanded={advancedOpen || advancedActive}
					class="w-full flex items-center gap-1 px-3 py-1.5 text-[11px] font-semibold uppercase tracking-wider text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 rounded">
					<ChevronRight class="w-3 h-3 transition-transform {advancedOpen || advancedActive ? 'rotate-90' : ''}" aria-hidden="true" />
					{i18n.t('nav.advanced')}
				</button>
			{/if}
			{#if advancedOpen || advancedActive || collapsed}
				<div class="space-y-0.5 mt-1">
					{#each advancedItems as item (item.href)}
						{@render navLink(item)}
					{/each}
				</div>
			{/if}
		</div>
	</nav>

	<div class="border-t border-gray-200 dark:border-gray-700 p-3 {collapsed ? 'px-2' : ''}">
		{#if !collapsed}
			<TodayUsageWidget />
		{/if}
		{@render navLink({ href: '/settings', label: i18n.t('nav.settings'), icon: Settings }, isSettingsActive)}
		<button onclick={toggleCollapse}
			class="flex items-center w-full rounded-lg text-sm font-medium transition-colors mt-1 {collapsed ? 'justify-center px-0 py-2' : 'gap-3 px-3 py-2'} text-gray-500 hover:bg-gray-100 hover:text-gray-700 dark:text-gray-400 dark:hover:bg-gray-700/50 dark:hover:text-gray-200"
			title={collapsed ? i18n.t('sidebar.expandTitle') : i18n.t('sidebar.collapseTitle')}>
			{#if collapsed}
				<PanelLeftOpen class="w-5 h-5 shrink-0" aria-hidden="true" />
			{:else}
				<PanelLeftClose class="w-5 h-5 shrink-0" aria-hidden="true" />
				<span>{i18n.t('sidebar.collapse')}</span>
			{/if}
		</button>
		{#if !collapsed}
			<p class="text-xs text-gray-500 dark:text-gray-400 px-3 mt-2">{appVersion ? `v${appVersion}` : ''}</p>
		{/if}
	</div>
</aside>

{#snippet navLink(item: NavItem, forceActive?: boolean)}
	{@const active = forceActive ?? isActive(item)}
	<a href={item.href} title={collapsed ? item.label : undefined} aria-current={active ? 'page' : undefined}
		class="flex items-center rounded-lg text-sm font-medium transition-colors {collapsed ? 'justify-center px-0 py-2 relative' : 'gap-3 px-3 py-2'}
			{active ? 'bg-primary-50 text-primary-700 dark:bg-primary-900/50 dark:text-primary-200' : 'text-gray-600 hover:bg-gray-100 dark:text-gray-300 dark:hover:bg-gray-700/50'}">
		<item.icon class="w-5 h-5 shrink-0" aria-hidden="true" />
		{#if !collapsed}
			<span class="whitespace-nowrap overflow-hidden flex-1">{item.label}</span>
		{/if}
		{#if item.badge}
			<span class="{collapsed ? 'absolute -top-0.5 -right-0.5' : ''} min-w-5 h-5 px-1.5 rounded-full bg-amber-100 text-amber-800 dark:bg-amber-900/60 dark:text-amber-200 text-[11px] font-semibold flex items-center justify-center"
				aria-label={i18n.t('sync.home.divergences', { n: item.badge })}>{item.badge}</span>
		{/if}
	</a>
{/snippet}
