# Estado (atualizado em 2026-11-23 22:22 por antigravity-cli)

## Fazendo agora

Unidade 2 concluída e verificada; pronto para iniciar a **Unidade 3** (Rebrand Visual `ZEROONE.GG` + Logo + Ícones Nativos Linux/Windows + Identidade `ZeroOne Agents Tools`).

## Próximo passo exato

Executar **Unidade 3**:
1. Copiar os assets necessários de `/home/felipe-nogueira/Workspace/projects/site-zeroone-linux/zeroone-brand-kit/` para dentro do repositório (`static/` / `src-tauri/icons/`).
2. Gerar ícones nativos Linux/macOS/Windows (`32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.ico`, `icon.icns`) via `npx tauri icon`.
3. Atualizar identidade em `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` (`productName: "ZeroOne Agents Tools"`, `identifier: "com.zeroone.agents-tools"`), títulos de janela e sidebar/header com a logo e paleta `ZEROONE.GG` (`#09090b`, `#111113`, `#1a1a1f`, `#8b5cf6`, `#22d3ee`, `Space Grotesk` / `Inter` / `JetBrains Mono`).

Seguido pela **Unidade 4**:
- Ajustar `.github/workflows/release.yml` para gerar instaladores **Windows** (`.exe`/`.msi`) e **Linux** (`.deb`/`.AppImage`/`.rpm`) no GitHub Actions sem depender de certificados Apple, gerar par de chaves do Tauri Updater, criar README de instalação para terceiros, e criar/publicar o repo público `Felipenogue91/zeroone-agents-tools` (e gerar o `.deb` local para instalar nesta máquina).

## Decisões em aberto

- Nenhuma bloqueante para Unidade 3. Publicação no GitHub (`Felipenogue91/zeroone-agents-tools`) já pré-aprovada para a Unidade 4.

## Feito e verificado

- **Unidade 1:** Fork clonado em `/home/felipe-nogueira/Workspace/projects/zeroone-agents-tools`, `upstream` configurado, `LICENSE` (MIT), `AGENTS.md` e `docs/estado/main.md`.
- **Unidade 2:**
  - Dependências nativas Tauri instaladas no sistema (`libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `libxdo-dev`, `patchelf`).
  - `src-tauri/src/utils/gemini_paths.rs` + `src-tauri/src/services/gemini_config.rs`: suporte cross-platform a `antigravity-cli/mcp_config.json` e campo `serverUrl` (`http`).
  - `src-tauri/src/db/schema.rs`: `get_all_projects` e `get_project_by_id` agora carregam `assigned_mcps` reais via `load_assigned_mcps_for_project` (corrigindo `list_projects` do MCP).
  - `src-tauri/src/services/scanner.rs` + `src-tauri/src/commands/scanner.rs`:
    - `run_full_scan` compartilhado entre startup e `scan_claude_directory`.
    - `scan_claude_json` ignora `$HOME` raiz como projeto.
    - `get_or_create_mcp`, `get_or_create_skill`, `get_or_create_hook` atualizam registros não-manuais quando mudam no disco.
    - `parse_hooks_from_settings` evita colisão de nomes em `matcher: ""` e matchers repetidos.
    - `scan_global_skills` preenche e reconcilia `global_skills`; `scan_agents_skills_dir` lê `<home>/.agents/skills`; `scan_global_mcps_from_claude_json` e `scan_global_hooks` limpam itens removidos; `reconcile_filesystem_entries` remove projetos e assets `auto-detected` apagados do disco preservando `manual`.
  - `src/lib/components/projects/ProjectToolsPanel.svelte` + `ProjectDetail.svelte`: seção `Global (Inherited)` com badge `Global` e exclusão de globais ativos da lista `Available`.
  - **Verificação:** `cargo fmt --check` ok, `cargo test` **2028 passed / 0 failed**, `npm run test:run` ok, `npm run build` ok, `svelte-check` manteve os mesmos 11 erros pré-existentes (0 novos).
