# Plano: sync multi-CLI (Claude Code, Codex, agy) + reforma de UI

Diagnóstico feito em 2026-11-24 (Claude Code, Opus 5.5). Este documento é a fonte para a implementação. Validar cada evidência no código atual antes de mexer: linhas podem ter mudado.

## Objetivo do Felipe

Painel de controle único para organizar a estrutura de IAs (skills, plugins, MCPs, agents, hooks), global e por projeto, gerenciando direto na interface, com as alterações refletindo nos arquivos reais dos projetos. As 3 CLIs (Claude Code, Codex, agy/Antigravity CLI) são interligadas: muitas vezes um skill/MCP serve às 3 ao mesmo tempo.

## Como o setup real do Felipe funciona hoje (fora do app)

- `~/.agents/` é a fonte da verdade:
  - `~/.agents/skills/<nome>/` — skills canônicas. `~/.claude/skills/<nome>` são **symlinks** para cá. O agy lê via `~/.gemini/config/skills.json` (entrada `~/.agents/skills`).
  - `~/.agents/mcp/global.json` — MCPs globais no formato `.mcp.json` do Claude.
  - `~/.agents/mcp/sync.json` — projetos gerenciados, `pluginProvided`, `codexGlobalKeep`, `overrides` por CLI (escopo `global`, `*`, ou nome da pasta do projeto).
  - `~/.agents/mcp/secrets.env` — valores de `${VAR}` (0600). **Nunca ler/imprimir conteúdo.**
  - `~/.agents/bin/mcp-sync` (Python, ~580 linhas) — gera: Claude user scope (`~/.claude.json` via `claude mcp`), Codex global `~/.codex/config.toml`, Codex projeto `<proj>/.codex/config.toml` (só `${VAR}`, vai pro git), agy global `~/.gemini/config/mcp_config.json`, agy projeto `<proj>/.agents/mcp_config.json` (valor resolvido, fora do git). `--check` só confere; `--here` é hook de início de sessão.
  - `<proj>/.claude/settings.json` `disabledMcpjsonServers` = desligar no projeto nas 3 CLIs.
- Ler `~/.agents/bin/mcp-sync` inteiro antes da Fase 2: ele é a especificação de comportamento.

## Falhas encontradas

1. **Dois donos da mesma config.** O app não lê `~/.agents/mcp/global.json` (DB tinha 5 globais, global.json 8). `write_global_config` substitui `mcpServers` inteiro do `~/.claude.json` (`src-tauri/src/services/config_writer.rs` ~151). `write_codex_config` faz `mcp_servers.clear()` (`src-tauri/src/services/codex_config.rs` ~231): apaga `node_repl`, campos extras (`startup_timeout_sec`, `enabled_tools`…) e o marcador do mcp-sync. Só não estoura hoje porque `enabled_editors` não está setado (padrão só `claude_code`, `commands/settings.rs` `get_enabled_editors_from_db`).
2. **agy não suportado de verdade.** Lê `~/.gemini/antigravity-cli/mcp_config.json` (`src-tauri/src/utils/gemini_paths.rs` ~26), mas o editor `gemini` escreve `~/.gemini/settings.json` (Gemini CLI) e `<proj>/.gemini/settings.json` (`commands/config.rs`, `commands/projects.rs` ~452). O agy usa `~/.gemini/config/mcp_config.json` e `<proj>/.agents/mcp_config.json`. Existem 3 `mcp_config.json` divergentes na máquina (`~/.gemini/config/`, `~/.gemini/antigravity-cli/`, `~/.gemini/antigravity/`). Confirmar qual o agy instalado realmente lê antes de decidir.
3. **Skills: toggle quebra o compartilhamento (o mais grave).** `skill_writer.rs` só conhece `~/.claude/skills` e OpenCode. Desligar = `remove_dir_all` (remove o symlink). Religar = `write_skill_file` cria pasta real só com `SKILL.md` regerado → perde o link para `~/.agents`, perde `scripts/`/`references/`. Editar skill global escreve **através** do symlink e reescreve o frontmatter da fonte. Codex e agy ignorados. Sem "desligar só numa CLI".
4. **Modelo de dados.** Identidade = nome (duplicado `Neon`/`neon`; mesmo nome com config diferente por projeto colapsa). Sem matriz de destinos por CLI, sem overrides por CLI/projeto (o que o `sync.json` faz), sem variante por projeto (ex.: supabase `project_ref`). Proveniência perdida: 35/41 MCPs e 131/131 skills com `source = 'auto-detected'`.
5. **Projetos.** `assign_mcp_to_project` só grava no DB; disco depende do botão "Sync Config" (`commands/projects.rs` ~219 vs ~250). `editor_type` único por projeto. Desligar no projeto remove do `.mcp.json` em vez de usar `disabledMcpjsonServers`. Grava valores resolvidos em `.mcp.json` e `.codex/config.toml` (versionados).
6. **Segredos.** 14 MCPs com env/headers literais em texto puro no SQLite (`~/.local/share/com.zeroone.agents-tools/mcp_library.db`). Sem suporte a `${VAR}`/`secrets.env`.
7. **Scan poluído.** Importa MCPs do catálogo do marketplace (`~/.claude/plugins/marketplaces/**/.mcp.json`: example-server, imessage, fakechat, terraform…) como se instalados.
8. **Escrita frágil / estado velho.** `std::fs::write` direto em `~/.claude.json` (que o Claude Code reescreve o tempo todo). Sem file watcher.
9. **Resto do fork.** Dois diretórios de dados (`~/.local/share/com.claude-code-tool-manager.app` e `com.zeroone.agents-tools`). MCP `claude-code-tool-manager` (nome antigo) ainda nas configs do agy. Processo antigo na porta 23847 (ver `docs/estado/main.md`).
10. **Futuro.** Formatos de Codex/agy mudam rápido sem testes de contrato; sem diff/prévia antes de gravar; sem desfazer; plugins de Codex/agy (`pluginProvided`) não modelados.

## Decisão de arquitetura (DECIDIDA pelo Felipe em 2026-11-24)

**`~/.agents` é a fonte da verdade; o app é a UI e o motor sobre ela.** O banco SQLite do app vira cache/índice (favoritos, tags, estado de UI), nunca a fonte das configs.
- Fases 0–2: o app lê/escreve o formato do `~/.agents` (`global.json`, `sync.json`, `.mcp.json` do projeto, `disabledMcpjsonServers`) e, quando `~/.agents/bin/mcp-sync` existir, delega a geração a ele (`--check` para diff, execução normal para aplicar).
- Depois: portar o motor para Rust dentro do app (com subcomando CLI equivalente ao `--here` para o hook de sessão) e aposentar o script.
- App é público/multi-usuário: o layout `~/.agents` vira o padrão para qualquer usuário, detectado e criado sem hardcode de caminho (regras do `AGENTS.md`).

## Fases

Cada fase = uma unidade. Ao fechar cada uma: atualizar `docs/estado/main.md`, rodar testes, e pedir aprovação do Felipe para commit.

### Fase 0 — Estancar (só Rust, pode começar já)
- Modo "gerenciado externamente": se `~/.agents/mcp/` existir, não escrever MCP global/projeto nas CLIs; UI mostra aviso.
- Skills: toggle = criar/remover **link** (symlink no Claude); se o destino é symlink, nunca escrever através dele nem substituir por pasta real; nunca apagar a fonte.
- Corrigir caminhos do agy (após confirmar qual arquivo ele lê).
- Parar de escanear catálogo de marketplace; só plugins instalados/habilitados.
- Escrita atômica (temp + rename) em todos os writers de config.
- Script/migration para limpar entradas com nome antigo no DB (não editar configs do usuário sem aprovação).
- Aceite: `cargo test` verde; testes novos cobrindo symlink preservado no toggle e codex preservando chaves desconhecidas.

### Fase 1 — Modelo de dados
- Recurso (mcp/skill/agent/hook) com ID próprio, local canônico, **matriz de destinos** (CLI × global/projeto), overrides por CLI e por projeto, segredos como referência `${VAR}`.
- Tabela de ocorrências: onde cada recurso foi visto (arquivo, CLI, escopo).
- Migration do DB atual sem perder favoritos/tags.

### Fase 2 — Motor de sync
- Adaptador por CLI com interface `read → plan → diff → apply`.
- Preservar campos desconhecidos (extras do Codex, `codexGlobalKeep`).
- Desligar no projeto via `disabledMcpjsonServers`.
- `${VAR}` só em arquivos versionados; valor resolvido só nos não versionados (agy).
- Skills: fonte em `~/.agents/skills`, materializar por CLI (symlink Claude; `skills.json` agy; Codex: verificar se a versão instalada lê `.agents/skills`).
- Toda aplicação mostra prévia do diff e faz backup.

### Fase 3 — Watcher + tela de Saúde
- Watcher (crate `notify`) nos arquivos de config.
- Tela com divergências por CLI (no arquivo mas não na fonte; na fonte mas não no arquivo) com ações "importar" / "reaplicar".

### Fase 4 — Projetos
- Várias CLIs por projeto (remover `editor_type` único).
- Salvar automático (sem botão Sync) com toast "Salvo em N CLIs · Desfazer".
- Herança visível (global esmaecido "herdado", override por projeto). Suporte a worktrees.

### Fase 5 — Segredos
- Valores no `secrets.env` (0600) ou keyring do SO; DB só guarda referência; UI mascara.
- Migrar os literais atuais do DB (pedir aprovação antes).

### Fase 6 — Prevenção
- Fixtures sanitizadas de config real de cada CLI; testes de ida e volta (campo desconhecido não some).
- Aviso quando versão da CLI instalada for desconhecida.

### Fase 7 — Reforma de UI
Começar só depois que a sessão do antigravity commitar as mudanças de UI pendentes (Sidebar, rules, settings, i18n…) — conferir `git status` antes.
1. Navegação por tarefa: Início (Saúde) · Projetos · Biblioteca (MCPs, Skills, Agents, Hooks, Commands, Rules como abas) · CLIs · Configurações. Containers, Status Line, Analytics, Insights, Sessions, Comparison, Profiles, Memory em "Claude Code avançado". Hoje são 19 itens e cortam em 900px.
2. Chips por CLI em todo item `[Claude] [Codex] [agy]` com ícone + texto, togláveis (padrão Meta "publicar no FB + IG").
3. Início = saúde ("Tudo sincronizado ✓" / "3 divergências → resolver"), não 8 cards de contagem + banner.
4. Projeto: uma barra de abas só; herdados esmaecidos; auto-save com desfazer.
5. Lista densa (Linear/GitHub) no lugar de grade de cards: ícone, nome, descrição, origem, chips de CLI, escopo, menu.
6. Filtros por significado (ativo onde, origem, com problema); transporte stdio/SSE/HTTP vai pro detalhe.
7. Painel lateral de detalhe com prévia do diff.
8. Ctrl+K (command palette).
9. Copy: pt-BR por padrão (screenshots saíram em EN — checar detecção), tirar jargão ("Unified control plane", "Gateway"), tooltips nos ícones do header, onboarding sem "onde você usa Claude Code".
10. Design system: tokens, 8pt, escala 20/14/12, um acento, skeletons, empty states com ação, dark como padrão (ver `CLAUDE.md`), medir contraste AA do cinza secundário. Animações mínimas.

## Guardrails para quem implementar

- Testes nunca tocam o HOME real: usar `HOME`/diretórios temporários.
- Não rodar `mcp-sync` em modo de aplicação, nem escrever em `~/.claude.json`, `~/.codex/config.toml`, `~/.gemini/**`, `~/.agents/**` sem aprovação do Felipe naquela conversa. `mcp-sync --check` é ok.
- Nunca ler/imprimir `~/.agents/mcp/secrets.env` nem valores de tokens.
- Não empacotar/gerar release sem o Felipe pedir.
- Banco do app: inspecionar só com `sqlite3 -readonly "file:...?mode=ro"`.
- Verificação: `cargo test` (em `src-tauri/`), `npm run test:run`, `npm run check` (não introduzir erros novos; há 11 pré-existentes).
- Screenshots da UI: o mock `e2e/fixtures/tauri-mock.ts` funciona com Playwright; o chromium baixado pelo projeto não existe, usar `executablePath` de `~/.cache/ms-playwright/chromium-1255/chrome-linux64/chrome`.
