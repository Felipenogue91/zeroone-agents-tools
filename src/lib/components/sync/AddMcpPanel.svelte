<script lang="ts">
	import { syncStore } from '$lib/stores/syncStore.svelte';
	import { i18n } from '$lib/i18n';
	import SidePanel from './SidePanel.svelte';
	import type { Cli } from '$lib/types/sync';
	import { CLIS, CLI_LABEL, scopeLabel } from '$lib/types/sync';

	type Props = { open: boolean; projects: string[]; onClose: () => void; defaultProject?: string | null };
	let { open, projects, onClose, defaultProject = null }: Props = $props();

	const templates: Record<string, Record<string, unknown>> = {
		stdio: { type: 'stdio', command: 'npx', args: ['-y', 'pacote-do-mcp'] },
		http: { type: 'http', url: 'https://exemplo.com/mcp', headers: { Authorization: 'Bearer ${MEU_TOKEN}' } }
	};

	let name = $state('');
	let scope = $state<string>('global');
	let kind = $state<'stdio' | 'http'>('stdio');
	let config = $state(JSON.stringify(templates.stdio, null, 2));
	let error = $state<string | null>(null);
	let from = $state('');
	let clis = $state<Record<Cli, boolean>>({ claude: true, codex: true, agy: true });

	/** MCPs you already use somewhere: start from one of them. */
	let existing = $derived(
		syncStore.resources
			.filter((r) => r.kind === 'mcp' && r.config)
			.sort((a, b) => a.name.localeCompare(b.name) || scopeLabel(a.scope).localeCompare(scopeLabel(b.scope)))
	);
	let fromRes = $derived(existing.find((r) => r.uid === from) ?? null);

	function pickExisting(uid: string) {
		from = uid;
		const r = existing.find((x) => x.uid === uid);
		if (!r) return;
		name = r.name;
		config = JSON.stringify(r.config, null, 2);
		kind = r.transport === 'stdio' ? 'stdio' : 'http';
	}

	$effect(() => {
		if (open) {
			name = '';
			scope = defaultProject ?? 'global';
			kind = 'stdio';
			config = JSON.stringify(templates.stdio, null, 2);
			error = null;
		}
	});

	function pickTemplate(k: 'stdio' | 'http') {
		kind = k;
		config = JSON.stringify(templates[k], null, 2);
	}

	async function submit() {
		error = null;
		if (!/^[A-Za-z0-9._-]+$/.test(name)) {
			error = i18n.t('sync.add.badName');
			return;
		}
		let parsed: Record<string, unknown>;
		try {
			parsed = JSON.parse(config);
		} catch {
			error = i18n.t('sync.detail.invalidJson');
			return;
		}
		if (fromRes?.literalSecretKeys.length) {
			error = i18n.t('sync.add.fromHasLiteral');
			return;
		}
		const project = scope === 'global' ? null : scope;
		const out = await syncStore.setServer(name, parsed, project);
		if (!out) return;
		const off = CLIS.filter((c) => !clis[c]).map((cli) => ({ project, name, cli, enabled: false }));
		if (off.length) await syncStore.setTargets(off);
		onClose();
	}
</script>

<SidePanel {open} title={i18n.t('sync.lib.addMcp')} subtitle={i18n.t('sync.add.subtitle')} {onClose}>
	{#if existing.length}
		<label class="block space-y-1">
			<span class="text-body font-medium">{i18n.t('sync.add.from')}</span>
			<select class="input" value={from} onchange={(e) => pickExisting((e.currentTarget as HTMLSelectElement).value)}>
				<option value="">{i18n.t('sync.add.fromNone')}</option>
				{#each existing as r (r.uid)}<option value={r.uid}>{r.name} · {scopeLabel(r.scope)}</option>{/each}
			</select>
			<span class="text-meta">{i18n.t('sync.add.fromHint')}</span>
		</label>
	{/if}
	<label class="block space-y-1">
		<span class="text-body font-medium">{i18n.t('sync.add.name')}</span>
		<input class="input" bind:value={name} placeholder="meu-mcp" autocomplete="off" />
	</label>
	<label class="block space-y-1">
		<span class="text-body font-medium">{i18n.t('sync.add.where')}</span>
		<select class="input" bind:value={scope}>
			<option value="global">{i18n.t('sync.add.global')}</option>
			{#each projects as p (p)}<option value={p}>{p.split('/').filter(Boolean).pop()}</option>{/each}
		</select>
		<span class="text-meta">{scope === 'global' ? i18n.t('sync.add.globalHint') : i18n.t('sync.add.projectHint')}</span>
	</label>
	<fieldset class="space-y-1">
		<legend class="text-body font-medium">{i18n.t('sync.add.type')}</legend>
		<div class="flex gap-2">
			<button data-tip={i18n.t('tip.add.local')} type="button" class="btn {kind === 'stdio' ? 'btn-primary' : 'btn-secondary'}" aria-pressed={kind === 'stdio'} onclick={() => pickTemplate('stdio')}>{i18n.t('sync.add.local')}</button>
			<button data-tip={i18n.t('tip.add.remote')} type="button" class="btn {kind === 'http' ? 'btn-primary' : 'btn-secondary'}" aria-pressed={kind === 'http'} onclick={() => pickTemplate('http')}>{i18n.t('sync.add.remote')}</button>
		</div>
	</fieldset>
	<label class="block space-y-1">
		<span class="text-body font-medium">{i18n.t('sync.add.config')}</span>
		<textarea class="input font-mono text-xs h-48" bind:value={config} spellcheck="false"></textarea>
		<span class="text-meta">{i18n.t('sync.detail.secretsHint')}</span>
	</label>
	<fieldset class="space-y-1">
		<legend class="text-body font-medium">{i18n.t('sync.add.turnOnIn')}</legend>
		<div class="flex gap-3">
			{#each CLIS as cli (cli)}
				<label class="flex items-center gap-1.5 text-body cursor-pointer">
					<input type="checkbox" class="rounded border-gray-300 dark:border-gray-600" bind:checked={clis[cli]} />{CLI_LABEL[cli]}
				</label>
			{/each}
		</div>
	</fieldset>
	{#if error}<p class="text-xs text-red-600 dark:text-red-400" role="alert">{error}</p>{/if}
	{#snippet footer()}
		<button class="btn btn-ghost" onclick={onClose}>{i18n.t('sync.panel.cancel')}</button>
		<button data-tip={i18n.t('tip.add.submit')} class="btn btn-primary" onclick={submit}>{i18n.t('sync.add.submit')}</button>
	{/snippet}
</SidePanel>
