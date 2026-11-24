# Estado (atualizado em 2026-11-23 23:25 por antigravity-cli)

## Fazendo agora

Unidades 1 a 4 concluídas: rebrand ZeroOne, reconciliação do scanner, `.deb` local gerado e pipeline multi-plataforma (`Windows` + `Linux` + `macOS`) pronto no GitHub Actions (`v1.0.0`).

## Próximo passo exato

- Acompanhar a primeira run da tag `v1.0.0` em `https://github.com/Felipenogue91/zeroone-agents-tools/actions` para confirmar geração dos instaladores Windows (`.exe`, `.msi`) e Linux (`.deb`, `.AppImage`, `.rpm`) na página de Releases.
- Se o Felipe aprovar localmente: parar o processo antigo (`claude-code-tool-manager` na porta `23847`), remover o pacote antigo e instalar `src-tauri/target/release/bundle/deb/ZeroOne Agents Tools_1.0.0_amd64.deb`.

## Decisões em aberto

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
