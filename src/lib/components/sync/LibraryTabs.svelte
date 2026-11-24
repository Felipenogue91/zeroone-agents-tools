<script lang="ts">
	import { page } from '$app/stores';
	import { i18n } from '$lib/i18n';

	const tabs = [
		{ href: '/library', key: 'sync.tabs.all', exact: true },
		{ href: '/skills', key: 'nav.skills' },
		{ href: '/subagents', key: 'nav.agents' },
		{ href: '/hooks', key: 'nav.hooks' },
		{ href: '/commands', key: 'nav.commands' },
		{ href: '/rules', key: 'nav.rules' },
		{ href: '/library/catalog', key: 'sync.tabs.catalog' }
	];

	function isActive(href: string, exact?: boolean) {
		const p = $page.url.pathname;
		return exact ? p === href : p === href || p.startsWith(href + '/');
	}
</script>

<nav aria-label={i18n.t('sync.tabs.label')} class="flex items-center gap-1 px-6 border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 overflow-x-auto">
	{#each tabs as t (t.href)}
		<a href={t.href} aria-current={isActive(t.href, t.exact) ? 'page' : undefined}
			class="px-3 py-2.5 text-sm font-medium border-b-2 whitespace-nowrap transition-colors {isActive(t.href, t.exact)
				? 'border-primary-500 text-primary-700 dark:text-primary-300'
				: 'border-transparent text-gray-500 hover:text-gray-800 dark:text-gray-400 dark:hover:text-gray-100'}">
			{i18n.t(t.key as never)}
		</a>
	{/each}
</nav>
