# Estado (atualizado em 2026-11-24 22:20 por claude-code)

## Fazendo agora

Frente **interface didática** (pedido do Felipe em 2026-11-24 21:00): todo botão e aba com explicação ao parar o mouse (janelinha global `GlobalTooltip`, `data-tip`/`data-tip-title`/`data-tip-side`, 700 ms), dizendo o que faz e a dor que resolve. Feito: cofre no importar + cópia do Claude (f8609f3), menu lateral e cabeçalho (9be8389), telas principais (Início, Biblioteca, Projetos, CLIs, Contexto, Containers, Code Forest) e categorias de Configurações. Ajudante pra pôr explicação: scratchpad addtip.py (anchor único → data-tip na tag anterior + chaves pt/en). Telas avançadas feitas (perfis, permissões, memória, barra de status, analytics, marketplace, hooks, skills, agentes, comandos, regras, containers, configurações). Botão só de ícone mostra o aria-label. Cobertura: 160 explicações próprias + 38 automáticas; ~320 botões ficam só com o texto (Editar, Excluir, itens de lista). 3.21.1 empacotada (deb, AppImage, NSIS Windows) e instalada aqui via pkexec (dpkg confirma 3.21.1). Nada publicado. Próximo passo possível: explicar os ~320 botões genéricos restantes se o Felipe quiser; conferir as janelinhas no app instalado.


Frente **aba Contexto**: Claude Code, **Codex e agy feitos** (sem push). Aguardando o Felipe conferir em `/context` no `npm run tauri dev`.

Frente anterior **UI que organiza de verdade**: itens 1–11 feitos e commitados (sem push).

Histórico da frente: (Felipe testou a fase 7 e reprovou: mostra sem deixar ajustar). Varredura feita em 2026-11-24 17:10 no app rodando (`npm run tauri dev`, tauri-mcp). Problemas achados:

1. **Projeto > MCPs:** herdados do global aparecem com chips, mas não dá pra clicar (só "Mude na Biblioteca"). Precisa desligar um global só neste projeto, por CLI ou nas 3.
2. **Biblioteca é dois mundos:** "Tudo" usa a fonte nova (MCPs 29, Skills 37, Agentes 0) e as abas Skills/Agentes/Hooks/Comandos/Regras são as telas antigas do banco (Skills 131 = 37 globais + 94 de projetos misturados sem agrupar; Agentes 7). Números não batem.
3. **MCPs:** chips por linha parecem etiqueta; falta tabela com colunas Claude/Codex/agy, marcar coluna inteira, ligar nas 3, seleção múltipla, ordenar/filtrar por CLI e por projeto.
4. **Skills:** grade de 131 cards sem dizer de qual projeto vem, sem ligar por CLI, sem agrupar famílias (`aos-*`, `ai-memory-*`, `neon-*`).
5. **CLIs:** "N pendentes" sem lista; "Reaplicar" não diz que apaga o que só existe na CLI.
6. **Início:** "Diferente da fonte" não mostra o que difere; cartões das CLIs não filtram; lista longa sem agrupar por projeto.
7. **Projetos:** 136 projetos com lixo (`.paperclip/instances`, `Workspace/runs/...`), colunas `0 | 0 | 0` sem rótulo, chip "Claude" fixo, não separa gerenciado/não gerenciado.
8. **Catálogo de MCPs:** banco antigo, não chega nas CLIs.
9. **Regras:** mostra 0 porque a consulta de regras globais quebra (`ambiguous column name: created_at`).
10. **Containers:** só lista os criados pelo app; os do Docker (`docker ps -a`, 20+) não aparecem. Select branco no modo escuro.
11. **Pedido novo:** aba Code Forest (embutir o servidor local do `~/Workspace/projects/code-forest`, sem fundir repos).

Ordem de execução: 1 → 3 → 2/4 → 5/6 → 8 → 9 → 10 → 7 → 11.

## Próximo passo exato

- Felipe confere as três abas da tela Contexto. Ideia em aberto: no agy a soma "Respostas do agy" passa do crescimento real em mensagens longas (o raciocínio conta na saída mas não fica no contexto); dá pra descontar se ele achar confuso.
- Achados para o Felipe: na sessão medida (65,3 mil tokens), ~47% é fixo do Claude Code (a ferramenta Artifact sozinha ~9 mil); skills 7,5 mil; MCPs 9,5 mil (Higgsfield sozinho: 205 ferramentas e ~3,4 mil); hooks de início só 1,4 mil porque o Claude corta a saída do `onde-estamos.sh` (10,4 KB) para um trecho.

- Felipe confere as telas. Push da `main` e empacotar Linux + Windows só com ok dele.
- Em aberto dele: embutir a floresta dentro da janela principal exigiria liberar `frame-ancestors` no code-forest só para a origem do app.
- Pendente menor: tela Projetos ainda lista 137 pastas com lixo (`.paperclip`, `Workspace/runs`) e colunas `0 | 0 | 0` sem rótulo (item 7 da varredura).
- "Empresa" nas mensagens do Felipe = a CLI de cada empresa (Anthropic/OpenAI/Google).

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

- **Versão 3.21.0 empacotada (2026-11-24 20:30):** `.deb` e AppImage Linux em `src-tauri/target/release/bundle/`, instalador Windows NSIS cruzado daqui (`cargo-xwin`, `--runner cargo-xwin --target x86_64-pc-windows-msvc`) em `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/`. Gerados com `createUpdaterArtifacts: false` (sem assinatura do updater). Instalado aqui via `pkexec apt install` (dpkg confirma 3.21.0 no lugar da 1.0.0); nada foi publicado (main 27 à frente do origin).

- **Aba Code Forest embutida (2026-11-24 20:00):** a janela separada ficava branca porque o WebKitGTK quebrava no driver NVIDIA 595 (`libnvidia-eglcore`, divide error) ao desenhar WebGL. Conserto: `main.rs` liga `WEBKIT_DISABLE_DMABUF_RENDERER=1` no Linux (testado fora do app: sem ele crasha, com ele roda liso). A floresta agora é um iframe na área da aba; o code-forest libera `frame-ancestors` só pras origens do app (commit no repo code-forest). Verificado: iframe 1056×689, app conectado no servidor 4790, nenhum crash no kernel. Falta o Felipe olhar com os próprios olhos (print do bridge não captura iframe).

- **Aba Contexto, Codex e agy (2026-11-24 19:55, Claude Code):** `services/context_cli.rs`. Codex: lê `~/.codex/sessions/**/rollout-*.jsonl` (prompt do sistema, ferramentas nativas, lista de skills com caminho, AGENTS.md, MCPs adiados só com nome, saída dos hooks, `token_usage_record` medido). agy: lê `~/.gemini/antigravity-cli/conversations/<id>.db` em modo `immutable` (nunca toca o WAL); o último `gen_metadata` traz o pedido inteiro em protobuf (seções do prompt, ferramentas, mensagens com tokens exatos) e cada geração tem o uso medido; leitor de protobuf sem schema embutido; subagentes aparecem como "Subagente: nome". Hooks configurados: Codex `~/.codex/hooks.json` (+ projeto), agy `~/.gemini/config/hooks.json`; "Rodar agora" funciona nos três. Comandos ganharam `cli`. Medido × estimado nas sessões reais: Codex 21,6 × 25,3 mil; agy 14,7 × 15,6 mil e 20,0 × 22,7 mil (zeroone-agents-tools). Verificado: `cargo test` 2153, Vitest 1581, `check` no baseline (11), três abas conferidas no app via tauri-mcp.

- **Aba Contexto, mensagem por mensagem (2026-11-24 19:02, Claude Code):** pedido do Felipe ("e o que entra a cada mensagem?"). O parser acompanha cada mensagem (digitada, só imagem ou aviso de tarefa): contexto real depois dela (`usage` da última chamada + saída), crescimento real, nº de chamadas, compactação, e de onde veio (sua mensagem com imagens ~1,6 mil tokens cada, resultados por ferramenta, respostas do Claude pelos tokens de saída medidos com raciocínio, hooks, anexos, e o resto). Tela: seção "Mensagem por mensagem" com gráfico do tamanho do contexto, lista em ordem ou por maior crescimento, detalhe ao abrir. Achado: na sessão de 2026-11-24 19:48 UTC, a mensagem 7 cresceu +274 mil (158 chamadas, 131 Bash, 14 prints). Verificado: `cargo test` 2149, Vitest 1581, `check` no baseline, conferido no app.

- **Aba Contexto, Claude Code (2026-11-24 18:50, Claude Code):** backend `services/context_inspector.rs` (parser do log, lista de pastas onde o Claude foi usado, sessões) e `services/context_config.rs` (hooks configurados de settings global/projeto/local + plugins ativos; rodar hook por id com stdin de hook, timeout que mata o grupo de processos; corpo das skills/subagentes). Comandos `list_context_folders`, `list_context_sessions`, `get_session_context`, `list_context_hooks`, `run_context_hook`. Tela `/context` (menu "Contexto"): seletor de CLI/pasta/sessão, medido × estimado × diferença, barra por grupo, grupos que abrem com origem e tokens por item, "Durante a sessão", "Hooks configurados" (apareceu ou não no log, "Rodar agora" com confirmação) e "Só quando usa". Verificado: `cargo test` 2148 ok, Vitest 85/1581, `npm run check` no baseline (11), tela conferida no app via tauri-mcp e "Rodar agora" testado com o `onde-estamos.sh`.

- **Verificação final (2026-11-24 18:20):** `cargo test` 2136/2136 (+2 ignorados), Vitest 85 arquivos / 1581 testes, `npm run check` no baseline (11 erros antigos), e2e Playwright 46/46 (rodado com o headless shell 1255 já instalado, porque o Playwright 1.58 do projeto não baixa navegador no Ubuntu 26.04).

- **Item 11, aba Code Forest (2026-11-24, Claude Code):** item "Code Forest" no menu (só aparece se `~/Workspace/projects/code-forest/bin/code-forest` existe). A página sobe o servidor da floresta se preciso (`setsid bin/code-forest serve`, sobrevive ao app como o `code-forest open`) e abre a floresta numa **janela nativa** do app (`WebviewWindowBuilder`, label `code-forest`, `http://127.0.0.1:4790/`). Não é iframe porque a floresta manda `frame-ancestors 'none'` (protege o botão de poda); afrouxar isso no code-forest é decisão do Felipe. Nada do code-forest foi copiado para cá. Containers também subiu para o menu principal.

- **Itens 9 e 10 (2026-11-24, Claude Code):** Regras: consultas com join usavam `created_at` sem prefixo (ambíguo) e a tela ficava vazia; corrigido com `rule_fields_as("r")` + teste. (Na máquina não há `~/.claude/rules`, então "0 regras" é real.) Containers: seção "Containers do Docker nesta máquina" (`list_docker_containers`, `docker_container_action`, `docker_container_logs`; bollard `all: true`), agrupada por projeto do compose com pasta, estado, portas, iniciar/parar/reiniciar, logs e "iniciar/parar todos"; atualiza a cada 5 s. Seletores nativos seguem o tema escuro (`color-scheme: dark`).

- **Item 8, catálogo (2026-11-24, Claude Code):** aba "Catálogo de MCPs" saiu da Biblioteca e da busca (a rota `/library/catalog` fica, por causa do Agregador e dos e2e antigos). "Adicionar MCP" ganhou "Começar de um MCP que você já usa" (copia nome+config de qualquer escopo, recusa se tiver segredo literal) e "Ligar em" Claude/Codex/agy.

- **Item 5, CLIs e pendências (2026-11-24, Claude Code):** **bug corrigido:** a ação de UM item ("Reaplicar da fonte") regravava a CLI inteira e apagaria tudo que só existia nela; agora `engine::narrow_to_server` limita a mudança àquele servidor e àquele escopo (JSON do Claude/agy e TOML do Codex; teste `narrow_to_server_keeps_the_other_servers`). Tela de CLIs: resumo "+adicionar ~corrigir −remover", lista de cada pendência com o motivo (`DivergenceList.svelte`, também no Início), botões que dizem o que fazem ("Remover do agy", "Adicionar no Codex", "Igualar à fonte", "Trazer para a fonte"); "Igualar tudo à fonte" avisa em vermelho o que vai ser removido antes de aplicar.

- **Item 4, skills na tabela (2026-11-24, Claude Code):** aba Skills da Biblioteca agora é a mesma tabela (`/library?kind=skill`), com as skills de cada projeto conhecido (`<p>/.agents/skills` = Codex e agy leem direto; `<p>/.claude/skills` = Claude) agrupadas por projeto, e as estacionadas (`.skills-disabled`) listadas como desligadas em vez de sumirem. Ligar no Claude por projeto = link relativo `../../.agents/skills/<nome>`; desligar = tira o link ou estaciona a pasta real (nunca apaga). Abas da Biblioteca: MCPs · Skills · Agentes · Hooks · Comandos · Regras · Catálogo. `ResourceList.svelte` removido (testes migrados para `ResourceMatrix`). Na máquina real: 199 skills (17 globais + projetos).

- **Item 3, tabela de MCPs (2026-11-24, Claude Code):** `ResourceMatrix.svelte` na Biblioteca: colunas Claude/Codex/agy clicáveis, checkbox da coluna inteira (itens visíveis), ligar/desligar nas 3 por linha, seleção múltipla com barra "Ligar no / Desligar no", ordenar por nome/origem/CLI, agrupado por origem (Global, depois cada projeto), filtros por origem/projeto e "ligados/desligados no X" e "o X não carrega". Status real por célula (`sync/live.rs`: `claude mcp list`, `codex mcp list --json`, `agy mcp list`; só global, porque Codex e agy ignoram a pasta) e quadro "carregados pelas CLIs mas fora da fonte". Lote = `sync_set_targets` / `save_many` (um backup, um Desfazer). Testes novos: parsers do live e `save_many_is_one_undoable_set`.
- Observado 17:26: alguém fora do app mexeu em `~/.agents` (global.json 8→5 MCPs, skills 37→17, sem backup do app). Não fui eu.

- **Item 1, MCP herdado por projeto (2026-11-24, Claude Code):** chips dos herdados agora clicáveis + botão ligar/desligar. Claude grava `~/.claude.json` `projects[path].disabledMcpServers` (o mesmo que o `/mcp` do Claude grava); Codex grava `cliOff[projeto].codex` e o motor escreve o servidor com `enabled = false` no `.codex/config.toml` do projeto (só projeto gerenciado); agy não tem como (testado: `agy mcp list` ignora a pasta), chip travado com o motivo. Saúde não acusa a entrada desligada como "só na CLI". Teste novo `inherited_global_off_per_project_in_claude_and_codex`; `cargo test sync::` 116/116, Vitest ok, check no baseline.

- **App só em pt-BR (2026-11-24, Claude Code):** botão de idioma saiu do cabeçalho; build de release sempre em pt-BR (só dev/teste aceitam `locale` no localStorage, usado pelas suítes em inglês). Vitest 1580/1580, `npm run check` igual ao baseline (11 erros).

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
