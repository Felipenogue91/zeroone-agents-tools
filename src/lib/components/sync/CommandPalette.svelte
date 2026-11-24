<script lang="ts">
	import { goto } from '$app/navigation';
	import { Search, CornerDownLeft } from 'lucide-svelte';
	import { syncStore } from '$lib/stores/syncStore.svelte';
	import { projectsStore } from '$lib/stores';
	import { i18n } from '$lib/i18n';
	import { scopeLabel } from '$lib/types/sync';

	type Item = { id: string; label: string; hint: string; group: string; run: () => void };

	let open = $state(false);
	let query = $state('');
	let index = $state(0);
	let input = $state<HTMLInputElement | null>(null);

	export function show() {
		open = true;
		query = '';
		index = 0;
		if (syncStore.resources.length === 0) syncStore.loadResources();
		queueMicrotask(() => input?.focus());
	}

	function close() {
		open = false;
	}

	function go(href: string) {
		close();
		goto(href);
	}

	let items = $derived.by<Item[]>(() => {
		const pages: [string, string][] = [
			['/', 'nav.home'],
			['/projects', 'nav.projects'],
			['/library', 'nav.library'],
			['/clis', 'nav.clis'],
			['/settings', 'nav.settings'],
			['/library?kind=skill', 'nav.skills'],
			['/subagents', 'nav.agents'],
			['/hooks', 'nav.hooks'],
			['/commands', 'nav.commands'],
			['/rules', 'nav.rules'],
			['/profiles', 'nav.profiles'],
			['/statusline', 'nav.statusLine'],
			['/permissions', 'nav.permissions'],
			['/memory', 'nav.memory'],
			['/analytics', 'nav.analytics'],
			['/insights', 'nav.insights'],
			['/sessions', 'nav.sessions'],
			['/comparison', 'nav.comparison'],
			['/containers', 'nav.containers'],
			['/marketplace', 'nav.marketplace']
		];
		const out: Item[] = pages.map(([href, key]) => ({
			id: `page:${href}`,
			label: i18n.t(key as never),
			hint: href,
			group: i18n.t('sync.palette.pages'),
			run: () => go(href)
		}));
		out.push(
			{
				id: 'action:check',
				label: i18n.t('sync.palette.check'),
				hint: '',
				group: i18n.t('sync.palette.actions'),
				run: () => {
					close();
					goto('/');
					syncStore.loadHealth();
				}
			},
			{
				id: 'action:add',
				label: i18n.t('sync.lib.addMcp'),
				hint: '',
				group: i18n.t('sync.palette.actions'),
				run: () => go('/library?add=1')
			}
		);
		for (const r of syncStore.resources) {
			out.push({
				id: `res:${r.uid}`,
				label: r.name,
				hint: `${i18n.t(`sync.lib.kind.${r.kind}` as never)} · ${scopeLabel(r.scope)}`,
				group: i18n.t('sync.palette.resources'),
				run: () => go(`/library?open=${encodeURIComponent(r.uid)}`)
			});
		}
		for (const p of projectsStore.projects) {
			out.push({
				id: `proj:${p.id}`,
				label: p.name,
				hint: p.path,
				group: i18n.t('nav.projects'),
				run: () => go(`/projects/${p.id}`)
			});
		}
		return out;
	});

	let results = $derived.by(() => {
		const q = query.trim().toLowerCase();
		if (!q) return items.filter((i) => i.id.startsWith('page:') || i.id.startsWith('action:')).slice(0, 12);
		return items
			.map((i) => {
				const l = i.label.toLowerCase();
				const score = l === q ? 0 : l.startsWith(q) ? 1 : l.includes(q) ? 2 : i.hint.toLowerCase().includes(q) ? 3 : 9;
				return { i, score };
			})
			.filter((x) => x.score < 9)
			.sort((a, b) => a.score - b.score || a.i.label.localeCompare(b.i.label))
			.slice(0, 30)
			.map((x) => x.i);
	});

	$effect(() => {
		query;
		index = 0;
	});

	$effect(() => {
		const handler = () => show();
		window.addEventListener('open-command-palette', handler);
		return () => window.removeEventListener('open-command-palette', handler);
	});

	function onKey(e: KeyboardEvent) {
		if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
			e.preventDefault();
			open ? close() : show();
			return;
		}
		if (!open) return;
		if (e.key === 'Escape') {
			e.preventDefault();
			close();
		} else if (e.key === 'ArrowDown') {
			e.preventDefault();
			index = Math.min(index + 1, results.length - 1);
		} else if (e.key === 'ArrowUp') {
			e.preventDefault();
			index = Math.max(index - 1, 0);
		} else if (e.key === 'Enter') {
			e.preventDefault();
			results[index]?.run();
		}
	}
</script>

<svelte:window onkeydown={onKey} />

{#if open}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="fixed inset-0 z-[60] bg-black/40 flex items-start justify-center pt-[12vh] px-4" onclick={close}>
		<div class="w-full max-w-xl rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 shadow-2xl overflow-hidden"
			role="dialog" tabindex="-1" aria-modal="true" aria-label={i18n.t('sync.palette.label')} onclick={(e) => e.stopPropagation()}>
			<label class="flex items-center gap-3 px-4 border-b border-gray-200 dark:border-gray-700">
				<Search class="w-4 h-4 text-gray-400" aria-hidden="true" />
				<span class="sr-only">{i18n.t('sync.palette.label')}</span>
				<input bind:this={input} bind:value={query} class="flex-1 py-3 bg-transparent text-sm outline-none text-gray-900 dark:text-white placeholder-gray-400"
					placeholder={i18n.t('sync.palette.placeholder')} role="combobox" aria-expanded="true" aria-controls="palette-results"
					aria-activedescendant={results[index] ? `pal-${index}` : undefined} />
				<kbd>Esc</kbd>
			</label>
			<ul id="palette-results" role="listbox" class="max-h-[50vh] overflow-y-auto py-1">
				{#each results as item, i (item.id)}
					{#if i === 0 || results[i - 1].group !== item.group}
						<li class="px-4 pt-2 pb-1 text-[11px] uppercase tracking-wide font-semibold text-gray-500 dark:text-gray-400" role="presentation">{item.group}</li>
					{/if}
					<li id="pal-{i}" role="option" aria-selected={i === index}>
						<button class="w-full flex items-center gap-3 px-4 py-2 text-left text-sm {i === index ? 'bg-primary-50 dark:bg-primary-900/40' : ''}"
							onmouseenter={() => (index = i)} onclick={() => item.run()}>
							<span class="flex-1 truncate text-gray-900 dark:text-white">{item.label}</span>
							{#if item.hint}<span class="text-meta truncate max-w-[50%]">{item.hint}</span>{/if}
							{#if i === index}<CornerDownLeft class="w-3.5 h-3.5 text-gray-400" aria-hidden="true" />{/if}
						</button>
					</li>
				{:else}
					<li class="px-4 py-6 text-center text-meta">{i18n.t('sync.palette.empty')}</li>
				{/each}
			</ul>
		</div>
	</div>
{/if}
