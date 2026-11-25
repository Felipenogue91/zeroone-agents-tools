<script lang="ts">
	import type { SkillFile, SkillFileType, CreateSkillFileRequest } from '$lib/types';
	import { skillLibrary } from '$lib/stores';
	import { i18n } from '$lib/i18n';
	import { Plus, FileText, Code, Image, Trash2, Edit, Save, X, FolderOpen } from 'lucide-svelte';

	type Props = {
		skillId: number;
		skillName: string;
	};

	let { skillId, skillName }: Props = $props();

	let files = $state<SkillFile[]>([]);
	let isLoading = $state(true);
	let error = $state<string | null>(null);

	// Add/Edit state
	let isAdding = $state(false);
	let editingId = $state<number | null>(null);
	let newFileType = $state<SkillFileType>('reference');
	let newFileName = $state('');
	let newFileContent = $state('');
	let editFileName = $state('');
	let editFileContent = $state('');

	const fileTypeConfig = $derived<Record<SkillFileType, { label: string; singular: string; icon: typeof FileText; color: string; dir: string; ext: string }>>({
		reference: { label: i18n.t('skillComp.typeReferences'), singular: i18n.t('skillComp.typeReference'), icon: FileText, color: 'bg-blue-100 text-blue-700 dark:bg-blue-900/50 dark:text-blue-400', dir: 'references', ext: '.md' },
		asset: { label: i18n.t('skillComp.typeAssets'), singular: i18n.t('skillComp.typeAsset'), icon: Image, color: 'bg-purple-100 text-purple-700 dark:bg-purple-900/50 dark:text-purple-400', dir: 'assets', ext: '' },
		script: { label: i18n.t('skillComp.typeScripts'), singular: i18n.t('skillComp.typeScript'), icon: Code, color: 'bg-green-100 text-green-700 dark:bg-green-900/50 dark:text-green-400', dir: 'scripts', ext: '.sh' }
	});

	async function loadFiles() {
		isLoading = true;
		error = null;
		try {
			files = await skillLibrary.getSkillFiles(skillId);
		} catch (e) {
			error = String(e);
		} finally {
			isLoading = false;
		}
	}

	async function handleAdd() {
		if (!newFileName.trim() || !newFileContent.trim()) return;

		try {
			const request: CreateSkillFileRequest = {
				skillId,
				fileType: newFileType,
				name: newFileName.trim(),
				content: newFileContent
			};
			const file = await skillLibrary.createSkillFile(request);
			files = [...files, file];
			resetAddForm();
		} catch (e) {
			error = String(e);
		}
	}

	async function handleUpdate(id: number) {
		if (!editFileName.trim() || !editFileContent.trim()) return;

		try {
			const updated = await skillLibrary.updateSkillFile(id, editFileName.trim(), editFileContent);
			files = files.map(f => f.id === id ? updated : f);
			editingId = null;
		} catch (e) {
			error = String(e);
		}
	}

	async function handleDelete(id: number) {
		if (!confirm(i18n.t('skillComp.deleteFileConfirm'))) return;

		try {
			await skillLibrary.deleteSkillFile(id);
			files = files.filter(f => f.id !== id);
		} catch (e) {
			error = String(e);
		}
	}

	function startEdit(file: SkillFile) {
		editingId = file.id;
		editFileName = file.name;
		editFileContent = file.content;
	}

	function cancelEdit() {
		editingId = null;
		editFileName = '';
		editFileContent = '';
	}

	function resetAddForm() {
		isAdding = false;
		newFileName = '';
		newFileContent = '';
	}

	function getFilesByType(type: SkillFileType): SkillFile[] {
		return files.filter(f => f.fileType === type);
	}

	// Load files on mount
	$effect(() => {
		loadFiles();
	});
</script>

<div class="space-y-4">
	<div class="flex items-center justify-between">
		<div class="flex items-center gap-2">
			<FolderOpen class="w-5 h-5 text-gray-500" />
			<h3 class="text-sm font-medium text-gray-700 dark:text-gray-300">
				{i18n.t('skillComp.skillFiles')}
			</h3>
		</div>
		{#if !isAdding}
			<button data-tip={i18n.t('tip.skills.addFile')}
				type="button"
				onclick={() => isAdding = true}
				class="btn btn-secondary text-sm"
			>
				<Plus class="w-4 h-4 mr-1" />
				{i18n.t('skillComp.addFile')}
			</button>
		{/if}
	</div>

	<p class="text-xs text-gray-500 dark:text-gray-400">
		{i18n.t('skillComp.filesStoredIn')} <code class="px-1 bg-gray-100 dark:bg-gray-700 rounded">.claude/skills/{skillName}/</code>
	</p>

	{#if error}
		<div class="p-3 bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 rounded-lg text-sm">
			{error}
		</div>
	{/if}

	{#if isLoading}
		<div class="text-sm text-gray-500">{i18n.t('skillComp.loadingFiles')}</div>
	{:else}
		<!-- Add New File Form -->
		{#if isAdding}
			<div class="p-4 border border-gray-200 dark:border-gray-700 rounded-xl bg-gray-50 dark:bg-gray-800/50 space-y-4">
				<div class="flex items-center justify-between">
					<span class="text-sm font-medium text-gray-700 dark:text-gray-300">{i18n.t('skillComp.newFile')}</span>
					<button type="button" onclick={resetAddForm} class="p-1 text-gray-400 hover:text-gray-600">
						<X class="w-4 h-4" />
					</button>
				</div>

				<!-- File Type Selector -->
				<div class="flex gap-2">
					{#each Object.entries(fileTypeConfig) as [type, config]}
						<button
							type="button"
							onclick={() => newFileType = type as SkillFileType}
							class="flex-1 flex items-center justify-center gap-2 p-2 rounded-lg border-2 transition-all {newFileType === type ? 'border-primary-500 bg-primary-50 dark:bg-primary-900/20' : 'border-gray-200 dark:border-gray-700 hover:border-gray-300'}"
						>
							<svelte:component this={config.icon} class="w-4 h-4" />
							<span class="text-sm">{config.singular}</span>
						</button>
					{/each}
				</div>

				<!-- File Name -->
				<div>
					<label for="new-file-name" class="block text-xs font-medium text-gray-600 dark:text-gray-400 mb-1">
						{i18n.t('skillComp.fileName')}
					</label>
					<input
						id="new-file-name"
						type="text"
						bind:value={newFileName}
						class="input text-sm"
						placeholder={newFileType === 'reference' ? 'colors.md' : newFileType === 'script' ? 'build.sh' : 'tokens.json'}
					/>
					<p class="mt-1 text-xs text-gray-500">
						{i18n.t('skillComp.willBeSavedTo')} <code class="px-1 bg-gray-100 dark:bg-gray-700 rounded">{fileTypeConfig[newFileType].dir}/{newFileName || i18n.t('skillComp.filenamePlaceholder')}</code>
					</p>
				</div>

				<!-- File Content -->
				<div>
					<label for="new-file-content" class="block text-xs font-medium text-gray-600 dark:text-gray-400 mb-1">
						{i18n.t('skillComp.content')}
					</label>
					<textarea
						id="new-file-content"
						bind:value={newFileContent}
						rows={8}
						class="input text-sm font-mono resize-y"
						placeholder={i18n.t('skillComp.fileContentPlaceholder')}
					></textarea>
				</div>

				<div class="flex justify-end gap-2">
					<button type="button" onclick={resetAddForm} class="btn btn-secondary text-sm">
						{i18n.t('common.cancel')}
					</button>
					<button
						type="button"
						onclick={handleAdd}
						class="btn btn-primary text-sm"
						disabled={!newFileName.trim() || !newFileContent.trim()}
					>
						{i18n.t('skillComp.addFile')}
					</button>
				</div>
			</div>
		{/if}

		<!-- File List by Type -->
		{#each Object.entries(fileTypeConfig) as [type, config]}
			{@const typeFiles = getFilesByType(type as SkillFileType)}
			{#if typeFiles.length > 0}
				<div class="space-y-2">
					<div class="flex items-center gap-2">
						<div class="w-6 h-6 rounded-md {config.color} flex items-center justify-center">
							<svelte:component this={config.icon} class="w-3.5 h-3.5" />
						</div>
						<span class="text-sm font-medium text-gray-700 dark:text-gray-300">{config.label}</span>
						<span class="text-xs text-gray-500">({config.dir}/)</span>
					</div>

					<div class="space-y-2 ml-8">
						{#each typeFiles as file (file.id)}
							<div class="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
								{#if editingId === file.id}
									<!-- Edit Mode -->
									<div class="p-3 bg-gray-50 dark:bg-gray-800/50 space-y-3">
										<input
											type="text"
											bind:value={editFileName}
											class="input text-sm"
											placeholder={i18n.t('skillComp.editFileNamePlaceholder')}
										/>
										<textarea
											bind:value={editFileContent}
											rows={8}
											class="input text-sm font-mono resize-y"
										></textarea>
										<div class="flex justify-end gap-2">
											<button type="button" onclick={cancelEdit} class="btn btn-secondary text-sm">
												<X class="w-4 h-4 mr-1" />
												{i18n.t('common.cancel')}
											</button>
											<button type="button" onclick={() => handleUpdate(file.id)} class="btn btn-primary text-sm">
												<Save class="w-4 h-4 mr-1" />
												{i18n.t('common.save')}
											</button>
										</div>
									</div>
								{:else}
									<!-- View Mode -->
									<div class="flex items-center justify-between p-3 bg-white dark:bg-gray-800">
										<div class="flex items-center gap-2">
											<code class="text-sm text-gray-700 dark:text-gray-300">{file.name}</code>
										</div>
										<div class="flex items-center gap-1">
											<button
												type="button"
												onclick={() => startEdit(file)}
												class="p-1.5 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 rounded hover:bg-gray-100 dark:hover:bg-gray-700"
												title={i18n.t('common.edit')}
											>
												<Edit class="w-4 h-4" />
											</button>
											<button
												type="button"
												onclick={() => handleDelete(file.id)}
												class="p-1.5 text-gray-400 hover:text-red-600 dark:hover:text-red-400 rounded hover:bg-red-50 dark:hover:bg-red-900/20"
												title={i18n.t('common.delete')}
											>
												<Trash2 class="w-4 h-4" />
											</button>
										</div>
									</div>
									<!-- Preview -->
									<div class="border-t border-gray-200 dark:border-gray-700 p-3 bg-gray-50 dark:bg-gray-900/50 max-h-32 overflow-auto">
										<pre class="text-xs text-gray-600 dark:text-gray-400 whitespace-pre-wrap font-mono">{file.content.slice(0, 500)}{file.content.length > 500 ? '...' : ''}</pre>
									</div>
								{/if}
							</div>
						{/each}
					</div>
				</div>
			{/if}
		{/each}

		{#if files.length === 0 && !isAdding}
			<div class="text-center py-6 text-gray-500 dark:text-gray-400">
				<FolderOpen class="w-8 h-8 mx-auto mb-2 opacity-50" />
				<p class="text-sm">{i18n.t('skillComp.noFilesYet')}</p>
				<p class="text-xs mt-1">{i18n.t('skillComp.noFilesHint')}</p>
			</div>
		{/if}
	{/if}
</div>
