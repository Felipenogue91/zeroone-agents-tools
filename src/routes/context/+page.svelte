<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { ChevronDown, RefreshCcw, Play, TriangleAlert, CircleCheck, CircleDashed, FolderOpen, Info } from 'lucide-svelte';
	import { Header } from '$lib/components/layout';
	import { i18n } from '$lib/i18n';

	type Item = {
		label: string;
		source: string | null;
		chars: number;
		tokens: number;
		note: string | null;
		count: number | null;
	};
	type Group = { key: string; title: string; hint: string; items: Item[]; chars: number; tokens: number };
	type Measured = { inputTokens: number; cacheCreationTokens: number; cacheReadTokens: number; total: number };
	type SessionContext = {
		sessionId: string;
		cwd: string | null;
		startedAt: string | null;
		claudeVersion: string | null;
		model: string | null;
		title: string | null;
		measured: Measured | null;
		startup: Group[];
		startupTokens: number;
		perTurn: Group[];
		prompts: number;
		onDemand: Group[];
		warnings: string[];
	};
	type SessionRef = { sessionId: string; modified: string | null; sizeBytes: number; title: string | null };
	type Folder = { path: string; sessions: number; lastUsed: string | null; exists: boolean };
	type Hook = {
		id: string;
		event: string;
		matcher: string | null;
		label: string;
		command: string;
		origin: string;
		timeoutSecs: number | null;
	};
	type HookRun = {
		exitCode: number | null;
		timedOut: boolean;
		elapsedMs: number;
		injectedChars: number;
		injectedTokens: number;
		stdoutChars: number;
		preview: string;
		stderr: string;
	};

	const CLIS = [
		{ key: 'claude', label: 'Claude Code', ready: true },
		{ key: 'codex', label: 'Codex', ready: false },
		{ key: 'agy', label: 'agy', ready: false }
	] as const;

	// One color per startup group, always paired with the name and number.
	const COLORS: Record<string, string> = {
		claude: 'bg-gray-400 dark:bg-gray-500',
		instructions: 'bg-blue-500',
		skills: 'bg-emerald-500',
		mcpInstructions: 'bg-violet-500',
		mcpTools: 'bg-fuchsia-400',
		agents: 'bg-cyan-500',
		hooks: 'bg-amber-500',
		session: 'bg-rose-400'
	};

	let cli = $state<'claude' | 'codex' | 'agy'>('claude');
	let folders = $state<Folder[]>([]);
	let folder = $state<string>(''); // '' = home folder
	let folderFilter = $state('');
	let sessions = $state<SessionRef[]>([]);
	let sessionId = $state<string>(''); // '' = most recent
	let ctx = $state<SessionContext | null>(null);
	let hooks = $state<Hook[]>([]);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let confirmHook = $state<string | null>(null);
	let runningHook = $state<string | null>(null);
	let runs = $state<Record<string, HookRun | string>>({});

	const fmt = (n: number) =>
		n >= 1000 ? `${(n / 1000).toLocaleString('pt-BR', { maximumFractionDigits: 1 })} mil` : n.toLocaleString('pt-BR');

	const fmtDate = (iso: string | null) =>
		iso ? new Date(iso).toLocaleString('pt-BR', { day: '2-digit', month: '2-digit', hour: '2-digit', minute: '2-digit' }) : '';

	const shownFolders = $derived.by(() => {
		const q = folderFilter.trim().toLowerCase();
		const list = q ? folders.filter((f) => f.path.toLowerCase().includes(q)) : folders;
		// keep the selected one visible even when filtered out
		if (folder && !list.some((f) => f.path === folder)) {
			const cur = folders.find((f) => f.path === folder);
			if (cur) return [cur, ...list];
		}
		return list;
	});

	const startupTotal = $derived(ctx?.startupTokens ?? 0);
	const measured = $derived(ctx?.measured?.total ?? null);
	const gap = $derived(measured !== null ? measured - startupTotal : null);

	/** Commands that showed up in the log (start or during the session). */
	const seenCommands = $derived.by(() => {
		const s = new Set<string>();
		for (const g of [...(ctx?.startup ?? []), ...(ctx?.perTurn ?? [])]) {
			if (g.key !== 'hooks') continue;
			for (const it of g.items) if (it.source) s.add(it.source);
		}
		return s;
	});

	const hooksByEvent = $derived.by(() => {
		const order = ['SessionStart', 'UserPromptSubmit', 'PreToolUse', 'PostToolUse'];
		const map = new Map<string, Hook[]>();
		for (const h of hooks) map.set(h.event, [...(map.get(h.event) ?? []), h]);
		return [...map.entries()].sort(
			(a, b) => (order.indexOf(a[0]) + 1 || 99) - (order.indexOf(b[0]) + 1 || 99) || a[0].localeCompare(b[0])
		);
	});

	const projectArg = () => (folder ? folder : null);

	async function loadFolders() {
		try {
			folders = await invoke<Folder[]>('list_context_folders');
		} catch (e) {
			error = String(e);
		}
	}

	async function loadSessions() {
		sessions = await invoke<SessionRef[]>('list_context_sessions', { projectPath: projectArg() });
	}

	async function loadContext() {
		loading = true;
		error = null;
		try {
			const [c, h] = await Promise.all([
				invoke<SessionContext | null>('get_session_context', {
					projectPath: projectArg(),
					sessionId: sessionId || null
				}),
				invoke<Hook[]>('list_context_hooks', { projectPath: projectArg() })
			]);
			ctx = c;
			hooks = h;
		} catch (e) {
			error = String(e);
			ctx = null;
		} finally {
			loading = false;
		}
	}

	async function pickFolder(path: string) {
		folder = path;
		sessionId = '';
		runs = {};
		try {
			await loadSessions();
		} catch (e) {
			error = String(e);
		}
		await loadContext();
	}

	async function pickSession(id: string) {
		sessionId = id;
		await loadContext();
	}

	async function refresh() {
		await Promise.all([loadFolders(), loadSessions().catch((e) => (error = String(e)))]);
		await loadContext();
	}

	async function runHook(id: string) {
		confirmHook = null;
		runningHook = id;
		try {
			runs[id] = await invoke<HookRun>('run_context_hook', { projectPath: projectArg(), hookId: id });
		} catch (e) {
			runs[id] = String(e);
		} finally {
			runningHook = null;
		}
	}

	const pct = (part: number, total: number) => (total > 0 ? Math.round((part / total) * 100) : 0);
	const maxTokens = (g: Group) => Math.max(1, ...g.items.map((i) => i.tokens));

	onMount(async () => {
		await loadFolders();
		// start on the most recently used folder
		folder = folders[0]?.path ?? '';
		await loadSessions().catch((e) => (error = String(e)));
		await loadContext();
	});
</script>

<Header title={i18n.t('nav.context')} subtitle={i18n.t('context.subtitle')} />

<div class="flex-1 overflow-y-auto">
	<div class="max-w-5xl mx-auto px-6 py-6 space-y-4">
		<!-- Pickers -->
		<section class="card space-y-3" aria-label={i18n.t('context.folder')}>
			<div class="flex flex-wrap items-center gap-2" role="tablist" aria-label={i18n.t('context.cli')}>
				{#each CLIS as c (c.key)}
					<button
						role="tab"
						aria-selected={cli === c.key}
						class="btn {cli === c.key ? 'btn-primary' : 'btn-secondary'}"
						onclick={() => (cli = c.key)}
					>
						{c.label}
						{#if !c.ready}<span class="ml-1.5 text-[11px] opacity-75">({i18n.t('context.soon')})</span>{/if}
					</button>
				{/each}
				<span class="flex-1"></span>
				<button class="btn btn-ghost" onclick={refresh} disabled={loading} title={i18n.t('context.refresh')}>
					<RefreshCcw class="w-4 h-4 mr-1.5 {loading ? 'animate-spin' : ''}" aria-hidden="true" />{i18n.t('context.refresh')}
				</button>
			</div>

			{#if cli === 'claude'}
				<div class="grid grid-cols-1 md:grid-cols-[1fr_1fr] gap-3">
					<div class="space-y-1">
						<label class="text-meta" for="ctx-folder">{i18n.t('context.folder')}</label>
						<input class="input input-sm" type="search" placeholder={i18n.t('context.filter')} bind:value={folderFilter} aria-label={i18n.t('context.filter')} />
						<select id="ctx-folder" class="input input-sm font-mono text-xs" value={folder} onchange={(e) => pickFolder(e.currentTarget.value)}>
							<option value="">{i18n.t('context.home')}</option>
							{#each shownFolders as f (f.path)}
								<option value={f.path}>{f.path} · {f.sessions} · {fmtDate(f.lastUsed)}{f.exists ? '' : ` (${i18n.t('context.missing')})`}</option>
							{/each}
						</select>
					</div>
					<div class="space-y-1">
						<label class="text-meta" for="ctx-session">{i18n.t('context.session')}</label>
						<select id="ctx-session" class="input input-sm" value={sessionId} onchange={(e) => pickSession(e.currentTarget.value)} disabled={sessions.length === 0}>
							<option value="">{i18n.t('context.latest')}</option>
							{#each sessions as s (s.sessionId)}
								<option value={s.sessionId}>{fmtDate(s.modified)} · {s.title ?? s.sessionId.slice(0, 8)}</option>
							{/each}
						</select>
						{#if ctx}
							<p class="text-meta">
								{[ctx.title, ctx.model, ctx.claudeVersion ? `Claude Code ${ctx.claudeVersion}` : null].filter(Boolean).join(' · ')}
							</p>
						{/if}
					</div>
				</div>
			{:else}
				<p class="text-body flex items-center gap-1.5"><Info class="w-4 h-4 text-gray-400 shrink-0" aria-hidden="true" />{i18n.t('context.soonBody')}</p>
			{/if}
		</section>

		{#if cli === 'claude'}
			{#if error}
				<div class="card border-red-300 dark:border-red-800 text-body text-red-700 dark:text-red-300" role="alert">
					<p class="font-medium">{i18n.t('context.loadFailed')}</p>
					<p class="text-meta break-words">{error}</p>
				</div>
			{:else if loading && !ctx}
				<div class="card text-body" aria-busy="true">{i18n.t('context.loading')}</div>
			{:else if !ctx}
				<div class="card text-body flex items-center gap-2"><FolderOpen class="w-4 h-4 text-gray-400" aria-hidden="true" />{i18n.t('context.noSessions')}</div>
			{:else}
				{#each ctx.warnings as w (w)}
					<p class="card text-body text-amber-700 dark:text-amber-300 flex items-start gap-2"><TriangleAlert class="w-4 h-4 mt-0.5 shrink-0" aria-hidden="true" />{w}</p>
				{/each}

				<!-- Totals -->
				<section class="grid grid-cols-1 sm:grid-cols-3 gap-3" aria-label={i18n.t('context.legend')}>
					<div class="card">
						<p class="text-meta">{i18n.t('context.measured')}</p>
						<p class="text-2xl font-semibold tabular-nums">{measured !== null ? fmt(measured) : '—'}</p>
						<p class="text-meta">{measured !== null ? i18n.t('context.measuredHint') : i18n.t('context.noMeasure')}</p>
					</div>
					<div class="card">
						<p class="text-meta">{i18n.t('context.estimated')}</p>
						<p class="text-2xl font-semibold tabular-nums">{fmt(startupTotal)}</p>
						<p class="text-meta">{i18n.t('context.estimatedHint')}</p>
					</div>
					<div class="card">
						<p class="text-meta">{i18n.t('context.gap')}</p>
						<p class="text-2xl font-semibold tabular-nums">{gap !== null ? (gap >= 0 ? '+' : '−') + fmt(Math.abs(gap)) : '—'}</p>
						<p class="text-meta">{i18n.t('context.gapHint')}</p>
					</div>
				</section>

				<!-- Startup -->
				<section class="card space-y-3" aria-labelledby="ctx-startup">
					<div>
						<h2 id="ctx-startup" class="text-title">{i18n.t('context.startupTitle')}</h2>
						<p class="text-meta">{i18n.t('context.startupBody')}</p>
					</div>
					<div class="flex h-3 w-full overflow-hidden rounded-full bg-gray-100 dark:bg-gray-800" aria-hidden="true">
						{#each ctx.startup as g (g.key)}
							<div class="{COLORS[g.key] ?? 'bg-gray-300'} h-full" style="width: {pct(g.tokens, startupTotal)}%" title="{g.title}: {fmt(g.tokens)}"></div>
						{/each}
					</div>
					<div class="space-y-2">
						{#each ctx.startup as g (g.key)}
							{@render groupBlock(g, startupTotal, COLORS[g.key])}
						{/each}
					</div>
				</section>

				<!-- During the session -->
				<section class="card space-y-3" aria-labelledby="ctx-turn">
					<div>
						<h2 id="ctx-turn" class="text-title">{i18n.t('context.turnTitle')}</h2>
						<p class="text-meta">{i18n.t('context.turnBody', { n: ctx.prompts })}</p>
					</div>
					{#if ctx.perTurn.length === 0}
						<p class="text-body">{i18n.t('context.turnEmpty')}</p>
					{:else}
						<div class="space-y-2">
							{#each ctx.perTurn as g (g.key)}
								{@render groupBlock(g, 0, undefined)}
							{/each}
						</div>
					{/if}
				</section>

				<!-- Configured hooks -->
				<section class="card space-y-3" aria-labelledby="ctx-hooks">
					<div>
						<h2 id="ctx-hooks" class="text-title">{i18n.t('context.hooksTitle')}</h2>
						<p class="text-meta">{i18n.t('context.hooksBody')}</p>
					</div>
					{#if hooks.length === 0}
						<p class="text-body">{i18n.t('context.noHooks')}</p>
					{:else}
						{#each hooksByEvent as [event, list] (event)}
							<div class="space-y-1.5">
								<h3 class="text-sm font-semibold text-gray-700 dark:text-gray-200">{event}</h3>
								<ul class="space-y-1.5">
									{#each list as h (h.id)}
										{@const run = runs[h.id]}
										<li class="rounded-lg border border-gray-200 dark:border-gray-700 px-3 py-2 space-y-1.5">
											<div class="flex items-start gap-2 flex-wrap">
												{#if seenCommands.has(h.command)}
													<CircleCheck class="w-4 h-4 mt-0.5 text-emerald-500 shrink-0" aria-label={i18n.t('context.hookSeen')} />
												{:else}
													<CircleDashed class="w-4 h-4 mt-0.5 text-gray-400 shrink-0" aria-label={i18n.t('context.hookSilent')} />
												{/if}
												<div class="flex-1 min-w-0">
													<p class="text-body font-medium">
														{h.label}
														{#if h.matcher}<span class="chip border-gray-300 text-gray-600 dark:border-gray-600 dark:text-gray-300 ml-1">{h.matcher}</span>{/if}
													</p>
													<p class="text-meta">{h.origin} · {seenCommands.has(h.command) ? i18n.t('context.hookSeen') : i18n.t('context.hookSilent')}</p>
													<p class="font-mono text-[11px] text-gray-500 dark:text-gray-400 break-all">{h.command}</p>
												</div>
												{#if confirmHook === h.id}
													<div class="flex items-center gap-2">
														<button class="btn btn-ghost" onclick={() => (confirmHook = null)}>{i18n.t('context.cancel')}</button>
														<button class="btn btn-primary" onclick={() => runHook(h.id)}>{i18n.t('context.runConfirm')}</button>
													</div>
												{:else}
													<button class="btn btn-secondary" disabled={runningHook !== null} onclick={() => (confirmHook = h.id)}>
														<Play class="w-3.5 h-3.5 mr-1.5" aria-hidden="true" />{runningHook === h.id ? i18n.t('context.running') : i18n.t('context.run')}
													</button>
												{/if}
											</div>
											{#if confirmHook === h.id}
												<p class="text-meta text-amber-700 dark:text-amber-300 flex items-center gap-1.5"><TriangleAlert class="w-3.5 h-3.5" aria-hidden="true" />{i18n.t('context.runWarn')}</p>
											{/if}
											{#if typeof run === 'string'}
												<p class="text-meta text-red-700 dark:text-red-300">{run}</p>
											{:else if run}
												<p class="text-meta">
													{run.timedOut
														? i18n.t('context.runTimeout')
														: i18n.t('context.runResult', { tokens: fmt(run.injectedTokens), code: run.exitCode ?? '?', ms: run.elapsedMs })}
												</p>
												{#if run.injectedChars > 10000}
													<p class="text-meta text-amber-700 dark:text-amber-300">{i18n.t('context.runCut')}</p>
												{/if}
												{#if run.preview || run.stderr}
													<pre class="max-h-48 overflow-auto rounded-md bg-gray-50 dark:bg-gray-900 p-2 text-[11px] whitespace-pre-wrap break-words">{run.preview}{run.stderr ? `\n— stderr —\n${run.stderr}` : ''}</pre>
												{/if}
											{/if}
										</li>
									{/each}
								</ul>
							</div>
						{/each}
					{/if}
				</section>

				<!-- On demand -->
				{#if ctx.onDemand.length > 0}
					<section class="card space-y-3" aria-labelledby="ctx-demand">
						<div>
							<h2 id="ctx-demand" class="text-title">{i18n.t('context.demandTitle')}</h2>
							<p class="text-meta">{i18n.t('context.demandBody')}</p>
						</div>
						<div class="space-y-2">
							{#each ctx.onDemand as g (g.key)}
								{@render groupBlock(g, 0, undefined)}
							{/each}
						</div>
					</section>
				{/if}
			{/if}
		{/if}
	</div>
</div>

{#snippet groupBlock(g: Group, total: number, color: string | undefined)}
	{@const max = maxTokens(g)}
	<details class="group rounded-lg border border-gray-200 dark:border-gray-700">
		<summary class="flex items-center gap-3 px-3 py-2 cursor-pointer select-none list-none">
			<ChevronDown class="w-4 h-4 shrink-0 text-gray-400 transition-transform group-open:rotate-180" aria-hidden="true" />
			{#if color}<span class="w-2.5 h-2.5 rounded-sm shrink-0 {color}" aria-hidden="true"></span>{/if}
			<span class="flex-1 min-w-0">
				<span class="text-body font-medium">{g.title}</span>
				<span class="text-meta ml-1.5">{i18n.t('context.items', { n: g.items.length })}</span>
			</span>
			{#if total > 0}<span class="text-meta tabular-nums">{i18n.t('context.ofStartup', { pct: pct(g.tokens, total) })}</span>{/if}
			<span class="text-body font-semibold tabular-nums w-20 text-right">{fmt(g.tokens)}</span>
		</summary>
		<div class="px-3 pb-3 space-y-2">
			<p class="text-meta">{g.hint}</p>
			<ul class="divide-y divide-gray-100 dark:divide-gray-800">
				{#each g.items as it, i (i)}
					<li class="py-1.5 flex items-start gap-3">
						<div class="flex-1 min-w-0">
							<p class="text-sm text-gray-800 dark:text-gray-100 break-words">
								{it.label}
								{#if it.count}<span class="text-meta ml-1">{i18n.t('context.runs', { n: it.count })}</span>{/if}
							</p>
							{#if it.source && it.source !== it.label}<p class="font-mono text-[11px] text-gray-500 dark:text-gray-400 truncate" title={it.source}>{it.source}</p>{/if}
							{#if it.note}<p class="text-meta break-words">{it.note}</p>{/if}
						</div>
						<div class="w-28 shrink-0 pt-1.5" aria-hidden="true">
							<div class="h-1.5 rounded-full bg-gray-100 dark:bg-gray-800 overflow-hidden">
								<div class="h-full rounded-full {color ?? 'bg-primary-500'}" style="width: {Math.max(2, (it.tokens / max) * 100)}%"></div>
							</div>
						</div>
						<span class="text-sm tabular-nums w-16 text-right text-gray-700 dark:text-gray-200">{fmt(it.tokens)}</span>
					</li>
				{/each}
			</ul>
		</div>
	</details>
{/snippet}
