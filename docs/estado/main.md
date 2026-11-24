# Estado (atualizado em 2026-11-24 19:10 por claude-code)

## Fazendo agora

Frente **sync multi-CLI** (`docs/plano-sync-multi-cli.md`), executando as fases em sequência sem parar para aprovação (pedido do Felipe em 2026-11-24): commit por fase, sem push, sem empacotar. Fase 0 concluída.

## Próximo passo exato

- Fase 1 (modelo de dados): recurso com ID próprio, matriz de destinos CLI × escopo, overrides, segredos como `${VAR}`, tabela de ocorrências, migration sem perder favoritos/tags.
- Depois de todas as fases: o Felipe confere e empacota Linux + Windows juntos (não empacotar antes).
- Pendente de antes: acompanhar a run da tag `v1.0.0` e trocar o pacote antigo `claude-code-tool-manager` (porta `23847`) pelo novo `.deb` quando o Felipe aprovar.

## Decisões em aberto

- Fonte da verdade decidida: `~/.agents` (banco do app é só cache). Fase 7 (UI) liberada: as mudanças de UI/i18n do antigravity foram commitadas em `e11105b`.
- Config dos usuários ainda não foi tocada: o MCP `claude-code-tool-manager` (nome antigo) segue em `~/.gemini/config/mcp_config.json` e `~/.gemini/antigravity-cli/mcp_config.json`; limpar só com aprovação.

## Feito e verificado

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
