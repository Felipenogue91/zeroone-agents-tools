# Estado (atualizado em 2026-11-24 23:30 por claude-code)

## Fazendo agora

Frente **sync multi-CLI** (`docs/plano-sync-multi-cli.md`) concluída: fases 0 a 7 commitadas na `main` (sem push, sem empacotar). Aguardando o Felipe conferir para empacotar Linux + Windows juntos.

## Próximo passo exato

- Felipe confere o app (`npm run tauri dev`) e decide os pontos em aberto abaixo.
- Depois: empacotar Linux + Windows juntos (só com o ok dele), push da `main`.
- Pendente de antes: run da tag `v1.0.0` e troca do pacote antigo `claude-code-tool-manager` (porta `23847`).

## Decisões em aberto

- **Aplicar na máquina real:** nada foi aplicado nas configs do Felipe. `zeroone-agents-tools --sync-health` mostra 34 divergências reais (inclui `claude-code-tool-manager` no agy e cópias antigas no escopo local do Claude); revisar e aplicar pela tela Início só com aprovação.
- **Hook de sessão:** trocar `mcp-sync --here` por `zeroone-agents-tools --sync-here` (mesma saída, entende `cliOff`) ou manter o script — ver `docs/sync-cliOff.md`.
- **Incidente (2026-11-24 ~15:42):** um subcomando ainda não registrado (`--secrets-status`) abriu a janela do build de dev por ~10 min contra a pasta de dados real. Efeito só no banco (cache): migrations 25/26 e scan de início (saíram as 14 entradas do catálogo do marketplace, 41 → 27 MCPs; atribuições intactas; 5 MCPs passaram a `${VAR}`). Nenhum arquivo de config das CLIs mudou (conferido por mtime). Corrigido: argumento `--sync-*`/`--secrets-*` desconhecido agora mostra ajuda e sai com código 2.
- Keyring do SO não entrou: o cofre é o `secrets.env` 0600 (o crate de keyring puxa dbus/secret-service no Linux; avaliar se valer a pena).
- **`cliOff` vs `mcp-sync`:** o app grava `cliOff` no `sync.json` para desligar MCP numa CLI só; o script atual não conhece essa chave. Opções em `docs/sync-cliOff.md` (trocar o hook por `zeroone-agents-tools --sync-here` ou aplicar o patch sugerido no script).
- **Máquina real fora de sincronia (só leitura, nada aplicado):** `--sync-check` e `mcp-sync --check` concordam: global com Claude/agy divergentes (agy ainda tem `claude-code-tool-manager`, `figma`, `twenty`, `voice-studio`…), 4 projetos com `agy@` desatualizado e cópias antigas no escopo local do Claude. Aplicar (`mcp-sync` ou `zeroone-agents-tools --sync-apply`) só com aprovação do Felipe.
- Fonte da verdade decidida: `~/.agents` (banco do app é só cache). Fase 7 (UI) liberada: as mudanças de UI/i18n do antigravity foram commitadas em `e11105b`.
- Config dos usuários ainda não foi tocada: o MCP `claude-code-tool-manager` (nome antigo) segue em `~/.gemini/config/mcp_config.json` e `~/.gemini/antigravity-cli/mcp_config.json`; limpar só com aprovação.

## Feito e verificado

- **Fase 7 sync multi-CLI — UI (2026-11-24, Claude Code):**
  - Navegação por tarefa: Início · Projetos · Biblioteca · CLIs (+ Configurações no rodapé); Perfis, Permissões, Memória, Status Line, Analytics, Insights, Sessões, Comparação, Containers e Marketplace no grupo recolhível "Claude Code avançado". Selo de divergências no Início.
  - Início = Saúde (`src/lib/components/sync/HealthHome.svelte`): "Tudo sincronizado" / "N diferenças para revisar", situação por CLI com versão, segredos faltando, divergências agrupadas com ações; toda ação abre painel lateral com diff mascarado antes de aplicar.
  - Biblioteca (`/library`): lista densa sobre a fonte (`ResourceList`), chips por CLI togláveis com ícone + texto + `aria-pressed`, filtros por significado (ligado em, origem, com problema), painel de detalhe (config mascarada, fonte, ajustes por CLI, onde aparece, editar/remover), adicionar MCP. Abas da biblioteca sob o cabeçalho; página antiga virou `/library/catalog`.
  - CLIs (`/clis`): versão vs testada, arquivos de cada CLI, pendências e "Reaplicar" com prévia.
  - Projeto: uma barra de abas só (MCPs · Skills · Agentes · Comandos · Hooks · Configurações), sem CLI única e sem botão Sync; `ProjectSyncPanel` com próprios + herdados esmaecidos, chips por CLI, desligar nas 3, worktrees, ativar/parar sincronização. Toda gravação mostra toast "Salvo em N CLIs · Desfazer".
  - Ctrl+K (`CommandPalette`): páginas, ações, recursos e projetos, só teclado. Botão "Buscar… Ctrl K" no cabeçalho.
  - Copy: pt-BR por padrão (WebKitGTK reporta `en-US`; inglês só se escolhido), `aria-label` do cabeçalho traduzidos, "Gateway" → "Agregador", sem "Unified control plane", onboarding sem "onde você usa Claude Code". 182 chaves novas em `en`/`pt-BR`.
  - Design system em `src/app.css`: tokens de texto `text-title/body/meta` (20/14/12), grade 8pt, `skeleton`, `chip`, `row-dense`, `kbd`, um acento (violeta da marca). Contraste AA medido: texto secundário do modo escuro subiu de 3,9:1/2,4:1 para 6,2:1/5,0:1 (`gray-400`/`gray-500` só no `.dark`). Animações só de cor/hover.
  - Verificação: `cargo test` 2128/2128, Vitest 85/85 arquivos e 1582/1582 (novo `src/tests/components/sync.test.ts`), `npm run check` só com os 11 erros pré-existentes (276 avisos, igual ao baseline), `npm run build` ok, e2e Playwright com mock 46/46 (specs do catálogo apontam para `/library/catalog`; mock força `locale=en`). Screenshots conferidos em escuro, claro e 900px.
- **Fase 6 sync multi-CLI (2026-11-24, Claude Code):**
  - Fixtures sanitizadas em `src-tauri/src/sync/fixtures/` (estrutura real de `~/.claude.json` 2.1, `~/.codex/config.toml` 0.171 com `[tools]`, `[hooks.state]`, subtabela `.env`; agy com chaves extras; `global.json`/`sync.json` híbridos; `.mcp.json` pretty; cofre falso).
  - `src-tauri/src/sync/fixture_tests.rs`: Codex preserva toda tabela/chave/comentário fora do que gera; `~/.claude.json` mantém as outras chaves e a ordem; agy mantém chaves desconhecidas e solta o `claude-code-tool-manager`; arquivos versionados só com `${VAR}`; segunda execução não muda nada; edição de fonte ida e volta byte a byte; saúde nas fixtures; `mcp-sync --check` concorda nas fixtures.
  - `src-tauri/src/sync/versions.rs`: versão instalada vs testada (Claude 2.x, Codex 0.160–0.179, agy 1.5.x) com aviso; entra na saúde, no `--sync-health` e em `get_cli_versions`. Máquina real: as 3 dentro do testado.
  - Verificação: `cargo test` 2128/2128, Vitest 1568/1568, `npm run check` só com os 11 erros pré-existentes.
- **Fase 5 sync multi-CLI (2026-11-24, Claude Code):**
  - `src-tauri/src/sync/vault.rs`: cofre `~/.agents/mcp/secrets.env` (criado/forçado 0600, edição preserva comentários e ordem). Status só com nomes: definidos, referenciados, faltando, sem uso, permissão.
  - Banco guarda só referência: fim do scan troca valores que existem no cofre por `${VAR}` (`Database::redact_known_secrets`). `get_sync_resources` mascara literais antes de mandar pra UI; saúde inclui `missingSecrets` e permissão do cofre.
  - Migração de literais (banco e, opcionalmente, `global.json`/`.mcp.json`) com nome sugerido (`MCP_CHAVE`) ou reaproveitando o nome já no cofre; só roda com `confirm: true`. Heurística ignora `XAUTHORITY`/`*_PATH`/`*_FILE`/`*_URL`.
  - Comandos: `get_secrets_status`, `set_secret`, `fix_secrets_permissions`, `plan_secret_migration`, `apply_secret_migration`. Terminal: `--secrets-status`.
  - Máquina real (só leitura): cofre 0600, 13 definidos = 13 referenciados, 0 literais pendentes.
  - Verificação: `cargo test` 2118/2118, Vitest 1568/1568, `npm run check` só com os 11 erros pré-existentes.
- **Fase 4 sync multi-CLI (2026-11-24, Claude Code):**
  - `src-tauri/src/sync/projects.rs`: salvar = gravar a fonte e regenerar as CLIs do escopo num único conjunto de backup (um "Desfazer" volta tudo). Ações: adicionar/editar/remover MCP (`global.json` ou `.mcp.json`, edição cirúrgica), desligar nas 3 CLIs (`disabledMcpjsonServers`), desligar só no Claude de projeto (`settings.local.json`), desligar numa CLI (`cliOff` no `sync.json`, documentado em `docs/sync-cliOff.md`), gerenciar projeto (`projects` do `sync.json`).
  - Projeto deixa de ter CLI única: todo projeto gerenciado gera para as 3 CLIs, com chave por servidor. `project_view` mostra servidores próprios, globais herdados (`inherited`), sobrescrita de global, worktrees com status do agy e pendências por CLI.
  - `engine::BackupSet` (vários passos num backup só) e `jsonedit::set_key` (qualquer caminho, preserva estilo; corrigido corte de linha compartilhada).
  - Comandos: `get_project_sync`, `sync_set_server`, `sync_set_enabled`, `sync_set_target`, `sync_manage_project`.
  - Verificação: `cargo test` 2113/2113, Vitest 1568/1568, `npm run check` só com os 11 erros pré-existentes; `--sync-check` segue idêntico ao `mcp-sync --check` na máquina real.
- **Fase 3 sync multi-CLI (2026-11-24, Claude Code):**
  - `src-tauri/src/sync/watcher.rs`: watcher (`notify` 6) nas pastas de config (não recursivo, filtrado por nome, debounce 600 ms); no app emite `sync-changed`, atualiza o índice de ocorrências e se reinicia quando o `sync.json` muda.
  - `src-tauri/src/sync/health.rs`: divergências `OnlyInCli`, `MissingInCli`, `Drift`, `ClaudeLocalScope`, `AgySkillsFolder`, cada uma com ações (`Import`, `Reapply`, `RemoveLocalCopy`, `FixAgySkills`). Importar converte Codex/agy → formato Claude e devolve valor resolvido para `${VAR}`.
  - `src-tauri/src/sync/jsonedit.rs`: edição cirúrgica de `mcpServers` no estilo do arquivo (híbrido do `global.json` ou pretty do `.mcp.json`); ida e volta byte a byte conferida nos 7 arquivos-fonte reais (teste ignorado `real_sources_roundtrip`).
  - Comandos: `get_sync_health`, `health_action_preview`, `health_action_apply`. Terminal: `--sync-health [--json]`.
  - Máquina real (só leitura): 66 recursos, 34 divergências, coerentes com o `mcp-sync --check`.
  - Verificação: `cargo test` 2105/2105 (+2 ignorados manuais), Vitest 1568/1568, `npm run check` só com os 11 erros pré-existentes.
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
