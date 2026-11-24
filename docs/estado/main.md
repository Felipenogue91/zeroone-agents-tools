# Estado (atualizado em 2026-11-23 23:20 por antigravity-cli)

## Fazendo agora

Unidade 3 concluída e verificada; pronto para iniciar a **Unidade 4** (Chaves do Updater, Workflows Windows + Linux, README público, build `.deb` local e publicação `v1.0.0` em `Felipenogue91/zeroone-agents-tools`).

## Próximo passo exato

Executar **Unidade 4**:
1. Gerar par de chaves do Tauri Updater em `~/.tauri/zeroone-agents-tools.key` e atualizar `pubkey` + `endpoints` em `src-tauri/tauri.conf.json`.
2. Atualizar `.github/workflows/release.yml`, `build.yml` e `rust-tests.yml` para `ubuntu-24.04` + `windows-latest` + `macos-latest` (sem depender de certificado Apple) e remover `update-homebrew.yml`.
3. Reescrever `README.md` com instruções de download/instalação para Windows (`.exe`/`.msi`), Linux (`.deb`/`.AppImage`/`.rpm`) e macOS (`.dmg`), além de créditos MIT ao upstream.
4. Gerar o `.deb` local via `npx tauri build --bundles deb`, atualizar este estado e fazer commit da Unidade 4.
5. Criar o repo público `Felipenogue91/zeroone-agents-tools`, configurar `TAURI_SIGNING_PRIVATE_KEY` e dar push em `main` + tag `v1.0.0`.

## Decisões em aberto

- Instalação local do novo `.deb` e remoção do pacote antigo `claude-code-tool-manager` aguardam confirmação do Felipe após a Unidade 4 gerar o `.deb`.

## Feito e verificado

- **Unidade 1:** Fork clonado em `zeroone-agents-tools`, `upstream` configurado, `LICENSE` (MIT), `AGENTS.md` e `docs/estado/main.md`.
- **Unidade 2:** Reconciliação no scanner, herança `Global (Inherited)` por projeto, suporte a `~/.agents/skills` e Antigravity CLI `mcp_config.json`, `list_projects` com `assignedMcps` reais.
- **Unidade 3:**
  - Assets oficiais da ZeroOne copiados para `static/brand/` (`zeroone-icone-app.svg`, `zeroone-horizontal-fundo-escuro.svg`, `zeroone-horizontal-fundo-claro.svg`) e ícones nativos (`.ico`, `.icns`, `.png`, `Square*Logo.png`, `static/favicon.png`) gerados com `npx tauri icon`.
  - Identidade atualizada para `ZeroOne Agents Tools` (`com.zeroone.agents-tools`, versão `1.0.0`) em `package.json`, `package-lock.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` (com `[lib] name = "claude_code_tool_manager"`), `mcp_server/{tools,server}.rs` e `whatsNew.svelte.ts`.
  - Design system `ZEROONE.GG` aplicado em `src/app.css`, `src/app.html`, `Sidebar.svelte`, `+page.svelte`, `WelcomeHero.svelte` e `SettingsEditorSyncTab.svelte`.
  - Adicionado suporte completo ao idioma `pt-BR` (`PT`) e atualizado i18n/testes (`layout.test.ts`, `e2e/navigation.spec.ts`, `playwright.config.ts`).
  - **Verificação:** `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test` (**2028 passed; 0 failed**), `npm run test:run` (84/84 arquivos), `npm run build` ok, `svelte-check` manteve 11 erros pré-existentes (0 novos).
