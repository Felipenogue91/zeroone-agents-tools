# Estado (atualizado em 2026-11-24 20:30 por claude-code)

## Fazendo agora

Frente **sync multi-CLI** (`docs/plano-sync-multi-cli.md`), executando as fases em sequência sem parar para aprovação (pedido do Felipe em 2026-11-24): commit por fase, sem push, sem empacotar. Fases 0, 1 e 2 concluídas.

## Próximo passo exato

- Fase 3 (watcher + saúde): crate `notify` nos arquivos de config, comando de saúde com divergências por CLI (no arquivo e não na fonte / na fonte e não no arquivo) e ações importar/reaplicar.
- Depois de todas as fases: o Felipe confere e empacota Linux + Windows juntos (não empacotar antes).
- Pendente de antes: acompanhar a run da tag `v1.0.0` e trocar o pacote antigo `claude-code-tool-manager` (porta `23847`) pelo novo `.deb` quando o Felipe aprovar.

## Decisões em aberto

- **Máquina real fora de sincronia (só leitura, nada aplicado):** `--sync-check` e `mcp-sync --check` concordam: global com Claude/agy divergentes (agy ainda tem `claude-code-tool-manager`, `figma`, `twenty`, `voice-studio`…), 4 projetos com `agy@` desatualizado e cópias antigas no escopo local do Claude. Aplicar (`mcp-sync` ou `zeroone-agents-tools --sync-apply`) só com aprovação do Felipe.
- Fonte da verdade decidida: `~/.agents` (banco do app é só cache). Fase 7 (UI) liberada: as mudanças de UI/i18n do antigravity foram commitadas em `e11105b`.
- Config dos usuários ainda não foi tocada: o MCP `claude-code-tool-manager` (nome antigo) segue em `~/.gemini/config/mcp_config.json` e `~/.gemini/antigravity-cli/mcp_config.json`; limpar só com aprovação.

## Feito e verificado

- **Fase 2 sync multi-CLI (2026-11-24, Claude Code):**
  - `src-tauri/src/sync/engine.rs`: porta Rust do `mcp-sync` (`read → plan → diff → apply`): Claude user scope (patch só de `mcpServers` no `~/.claude.json`, relido na hora de gravar), Codex global (preserva `codexGlobalKeep`, extras e o resto do TOML) e projeto (só `${VAR}`, vai pro git), agy global e por pasta/worktree (valor resolvido, 0600, `info/exclude`). `pluginProvided`, `overrides`, `disabledMcpjsonServers`, avisos de escopo local do Claude.
  - `apply` com backup por conjunto (`<data>/com.zeroone.agents-tools/sync-backups/<id>`, 0700, últimos 30), pula arquivo que mudou desde a prévia e `undo` restaura. Diff unificado com segredos mascarados (`secrets.rs`).
  - `src-tauri/src/sync/codex.rs`: `to_codex`, render TOML e `codex_rewrite` idênticos ao script (mesmo marcador). `serde_json` com `preserve_order` (2 snapshots do insta reordenados, conteúdo igual).
  - `src-tauri/src/sync/skills.rs`: skill por CLI (symlink no Claude, `include_only` no `skills.json` do agy, Codex nativo).
  - Comandos: `sync_preview`, `sync_apply`, `sync_undo`, `list_sync_backups`, `set_skill_target`, `run_external_sync_check`. Terminal: `zeroone-agents-tools --sync-here` (hook de sessão), `--sync-check [--json]`, `--sync-apply`.
  - Paridade: teste roda o `mcp-sync --check` real com `HOME` na fixture depois do motor Rust aplicar → tudo "ok". Na máquina real, `--sync-check` gera saída idêntica ao `mcp-sync --check`.
  - Verificação: `cargo test` 2091/2091 (+1 ignorado manual), Vitest 1568/1568, `npm run check` só com os 11 erros pré-existentes.
- **Fase 1 sync multi-CLI (2026-11-24, Claude Code):**
  - `src-tauri/src/sync/model.rs`: `Resource` com `uid` próprio (`kind:escopo:nome`, sensível a maiúsculas, por projeto), `Cli` (claude/codex/agy), matriz de destinos (`On`/`Off`/`ProvidedByPlugin`/`Native`/`Unsupported`), overrides por CLI, `secret_refs` (`${VAR}`, nunca valor) e `literal_secret_keys`.
  - `src-tauri/src/sync/source.rs`: lê a fonte (`global.json`, `sync.json` ignorando `_doc`, `.mcp.json` + `disabledMcpjsonServers` dos projetos, `~/.agents/skills`, `~/.claude/skills`, agents). Codex lê `~/.agents/skills` nativamente (confirmado no binário 0.171: `user-agent-skills`); agy via `~/.gemini/config/skills.json`.
  - `src-tauri/src/sync/occurrences.rs`: índice de onde cada recurso aparece em cada arquivo de cada CLI (inclusive escopo local do Claude no `~/.claude.json`).
  - `src-tauri/src/db/resources.rs` + migration 26: tabelas `resource_meta` (favorito/tags/estado de UI por `uid`) e `resource_occurrences`; favoritos e tags antigos migrados por escopo. Scanner atualiza o índice no fim do scan.
  - Comandos: `get_sync_resources`, `refresh_sync_occurrences`, `set_resource_favorite`, `set_resource_tags`.
  - Conferido contra o `~/.agents` real (teste `real_home_snapshot`, ignorado por padrão, só nomes): 8 globais, 6 projetos, pluginProvided/overrides/desligados corretos, 37 skills.
  - Verificação: `cargo test` 2068/2068, Vitest 1568/1568, `npm run check` só com os 11 erros pré-existentes.
- **Fase 0 sync multi-CLI (2026-11-24, Claude Code):**
  - `src-tauri/src/utils/agents_home.rs`: detecta `~/.agents` sem caminho fixo. Se `~/.agents/mcp/` existe, MCP é "gerenciado externamente": `sync_global_config`, `sync_project_config`, `activate_profile` e o registro de projeto no `~/.claude.json` recusam escrever (mensagem pt-BR). Novo comando `get_mcp_management_status` para a UI.
  - `src-tauri/src/utils/atomic.rs`: escrita atômica (temp + rename, segue symlink, preserva permissão) em todos os writers de config (`config_writer`, `claude_json`, `codex`, `gemini`, `cursor`, `copilot`, `opencode`, skills, agents, hooks, rules, commands, settings…).
  - Codex: `write_codex_config` agora mescla. Preserva servidores que o app não gerencia (`node_repl`), chaves desconhecidas (`startup_timeout_sec`, `env_vars`, `enabled_tools`, `default_tools_approval_mode`), resto do TOML e o marcador do mcp-sync; desligado vira `enabled = false`.
  - Skills: toggle global = symlink para `~/.agents/skills/<nome>`; nunca escreve através de symlink; desligar remove só o link; pasta real desligada vai para `.claude/.skills-disabled/` (restaurável, nunca apagada); recusa remover a fonte canônica; escopo projeto copia a pasta inteira (sem link pessoal no git). Bug corrigido: `toggle_global_skill` selecionava 13 de 20 colunas e falhava depois de mudar o banco. Scanner não descarta mais skill global desligada.
  - agy: confirmado no binário 1.5.16 que o global é `~/.gemini/config/mcp_config.json`; `gemini_paths.antigravity_mcp_file` corrigido.
  - Scanner: plugins só de `installed_plugins.json` com `enabledPlugins` true (user ou projeto); parou de importar o catálogo do marketplace.
  - Migration 25 / `Database::cleanup_scan_pollution`: remove do banco os MCPs do catálogo e as entradas com nome antigo do próprio app, mantendo favoritos/atribuídos/system. Configs do usuário intocadas.
  - Verificação: `cargo test` 2051/2051, `npm run test:run` 84/84 arquivos e 1568/1568 testes, `npm run check` só com os 11 erros pré-existentes.

- **Unidade 1:** Fork clonado em `zeroone-agents-tools`, `upstream` configurado, `LICENSE` (MIT), `AGENTS.md` e `docs/estado/main.md`.
- **Unidade 2:** Reconciliação no scanner, herança `Global (Inherited)` por projeto, suporte a `~/.agents/skills` e Antigravity CLI `mcp_config.json`, `list_projects` com `assignedMcps` reais.
- **Unidade 3:** Rebrand visual `ZEROONE.GG`, logo/ícones nativos, `pt-BR`, `package.json`/`Cargo.toml`/`tauri.conf.json` em `1.0.0`, `cargo test` (2028/2028) e Vitest (84/84 arquivos, 1568/1568 testes) passando.
- **Unidade 4:**
  - Par de chaves do Tauri Updater gerado em `~/.tauri/zeroone-agents-tools.key` (fora do repositório) e `pubkey` + `endpoints` configurados em `src-tauri/tauri.conf.json`.
  - Workflows atualizados para `ubuntu-24.04`, `windows-latest` e `macos-latest` (`releaseDraft: false`, sem dependência de certificado Apple; `update-homebrew.yml` removido).
  - `README.md` reescrito com instruções de download e instalação para Windows (`.exe`/`.msi`), Linux (`.deb`/`.AppImage`/`.rpm`) e macOS (`.dmg`) + atribuição MIT ao upstream.
  - Build local `.deb` concluído e assinado: `src-tauri/target/release/bundle/deb/ZeroOne Agents Tools_1.0.0_amd64.deb` (15 MB) + `.sig`.
  - Repositório público `Felipenogue91/zeroone-agents-tools` criado, secret `TAURI_SIGNING_PRIVATE_KEY` configurado e tag `v1.0.0` disparada.
- **Unidade 5 (2026-11-24):**
  - Dicionário `pt-BR` completado em `src/lib/i18n/locales/pt-BR.ts` (de 170 para **2.033 chaves**, 100% em paridade com `src/lib/i18n/locales/en.ts`; `zh-CN.ts` e `zh-TW.ts` com fallback `...en`).
  - Plugados em `i18n.t(...)`: `Sidebar.svelte`, `WelcomeHero.svelte`, rotas `rules`, `containers` (`+page`, `hosts`, `templates`), `settings` (`+page`, `settingsCategories.ts`, `ScopedSettingsWrapper.svelte`) e todos os componentes internos de `projects`, `mcp`, `subagents`, `skills`, `profiles`, `statusline`, `permissions`, `memory`, `plugins`, `analytics`, `comparison`, `sessions`, `settings/tabs`, editores de settings, `hooks`/`sounds` e `containers`.
  - Verificação pós-edição: auditoria `en`/`pt-BR` (2.033/2.033 chaves, 0 ausentes), `npm run test:run` (84/84 arquivos, 1568/1568 testes passando), `npm run build` OK e `npm run check` apenas com os 11 erros pré-existentes do upstream. Sem empacotamento ainda (aguardando outra sessão).
