# Estado (atualizado em 2026-11-23 22:07 por antigravity-cli)

## Fazendo agora

Unidade 1 concluída; aguardando ok do Felipe nas permissões para iniciar a Unidade 2.

## Próximo passo exato

Iniciar **Unidade 2**: instalar os pacotes `-dev` locais do Tauri (`libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev patchelf`) e corrigir o motor em Rust + Svelte:
1. Scanner reconciliador (atualizar itens alterados e limpar do SQLite itens `auto-detected` cujo arquivo/entrada não existe mais no disco, preservando `manual`).
2. `scan_global_skills` populando `global_skills` automaticamente.
3. `parse_hooks_from_settings` incluindo índice externo `idx` para eliminar colisão de nomes.
4. Tela de Projetos (`ProjectToolsPanel.svelte`) exibindo seção/badge clara de **Globais (herdados)** em vez de listar MCPs/skills globais como "Available".
5. MCP `list_projects` retornando `assignedMcps` reais.
6. Suporte cross-platform opcional a `<home>/.agents/skills` e Antigravity CLI (`<home>/.gemini/antigravity-cli/mcp_config.json`) quando existirem na máquina do usuário.

Seguido por:
- **Unidade 3:** Rebrand visual `ZEROONE.GG` + logo + ícones nativos `.ico`/`.png`/`.icns` via `npx tauri icon` + `identifier: com.zeroone.agents-tools` e `productName: ZeroOne Agents Tools`.
- **Unidade 4:** Workflow GitHub Actions `release.yml` ajustado para gerar instaladores Windows (`.exe`/`.msi`) + Linux (`.deb`/`.AppImage`) sem exigir certificado Apple, chave Tauri updater própria, README de instalação e publicação em `Felipenogue91/zeroone-agents-tools`.

## Decisões em aberto

1. Ok do Felipe para abrir prompt gráfico de senha (`pkexec apt-get install`) e instalar as 6 libs `-dev` do GTK/WebKit2GTK para compilar e testar o backend Rust/Tauri localmente.
2. Ok explícito para criar e publicar o repositório **público** `Felipenogue91/zeroone-agents-tools` no GitHub (para terceiros baixarem o fork e o GitHub Actions compilar os instaladores de Windows e Linux).

## Feito e verificado

- Clone da `v3.10.0` (`63b54d6`) em `/home/felipe-nogueira/Workspace/projects/zeroone-agents-tools`.
- Remote `origin` renomeado para `upstream` (sem `origin` configurado até aprovação de publicação).
- `LICENSE` (MIT), `AGENTS.md` e `docs/estado/main.md` criados.
- `npm ci` concluído (210 pacotes).
- `npm run test:run` passou: 84 arquivos, 1567 testes aprovados.
- `npm run build` passou (`build/` gerado com `@sveltejs/adapter-static`).
- Baseline observado: `npm run check` falha com 11 erros TS pré-existentes do upstream em `containers/` e `AgentMemoryPanel.svelte` (não bloqueiam `vite build`).
