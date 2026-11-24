# Estado (atualizado em 2026-11-24 14:28 por antigravity-cli)

## Fazendo agora

Unidades 1 a 5 concluídas: cobertura completa de `pt-BR` na interface visível (2.033 chaves em `en.ts` e `pt-BR.ts`, ~130 componentes/rotas plugados em `i18n`). Empacotamento pausado a pedido do Felipe até a outra sessão terminar as alterações de sync multi-CLI.

## Próximo passo exato

- Decidir se quer commitar as mudanças de UI/i18n agora (para liberar a Fase 7 da frente de sync multi-CLI) ou aguardar o fechamento da outra sessão antes de commitar e gerar os novos pacotes Linux/Windows.
- Acompanhar a primeira run da tag `v1.0.0` em `https://github.com/Felipenogue91/zeroone-agents-tools/actions` para confirmar geração dos instaladores Windows (`.exe`, `.msi`) e Linux (`.deb`, `.AppImage`, `.rpm`) na página de Releases.
- Se o Felipe aprovar localmente: parar o processo antigo (`claude-code-tool-manager` na porta `23847`), remover o pacote antigo e instalar o `.deb` atualizado após o próximo empacotamento.

## Decisões em aberto

- **Nova frente (2026-11-24, Claude Code):** plano de sync multi-CLI (Claude, Codex, agy) + reforma de UI em `docs/plano-sync-multi-cli.md`. Fonte da verdade decidida: `~/.agents` (banco do app é só cache). Começar pela Fase 0 (só Rust). Fase 7 (UI) só depois que as mudanças de UI pendentes da sessão do antigravity forem commitadas.

- Aguardando confirmação do Felipe para instalar o `.deb` novo localmente e remover o pacote antigo `claude-code-tool-manager`.

## Feito e verificado

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
