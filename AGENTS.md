# ZeroOne Agents Tools

Fork aberto (licença MIT) do `tylergraydev/claude-code-tool-manager` mantido pela ZeroOne para uso próprio e distribuição pública multi-usuário em **Linux** (`.deb`, `.AppImage`, `.rpm`) e **Windows** (`.exe`, `.msi`).

## Regras para multi-usuário e cross-platform

- **Nunca hardcodar caminhos pessoais** (como `/home/felipe-nogueira/...` ou caminhos Windows fixos), tokens, credenciais ou dados de máquina específica no código ou em arquivos versionados.
- Resolver todos os diretórios de usuário e configuração de forma cross-platform (`dirs::home_dir()`, `std::path::PathBuf`, variáveis de ambiente padrão) compatível com Linux e Windows.
- Recursos opcionais (como `.agents/skills` e Antigravity CLI) devem ser detectados apenas se existirem na máquina do usuário, sem erro nem ruído quando ausentes.

## Identidade visual (somente leitura)

- Referência oficial da marca (não fundir repositórios):
  - `/home/felipe-nogueira/Workspace/projects/site-zeroone-linux/zeroone-brand-kit/`
  - `/home/felipe-nogueira/Workspace/projects/site-zeroone-linux/src/styles/global.css`
- Copiar apenas os assets necessários para dentro deste repositório ao aplicar o rebrand (Unidade 3).

## Stack e comandos de verificação

- **Frontend:** Svelte 5 + SvelteKit + Tailwind CSS v4
  - `npm run test:run` — suíte Vitest (baseline `v3.10.0`: 84 arquivos / 1567 testes passando)
  - `npm run build` — build estático de produção (`build/`)
  - `npm run check` — `svelte-check` possui 11 erros TS pré-existentes do upstream em `src/lib/components/containers/*` e `src/lib/components/subagents/AgentMemoryPanel.svelte`; não introduzir novos erros nos arquivos modificados.
- **Backend nativo:** Tauri 2 + Rust (`src-tauri/`)
  - `cargo test` dentro de `src-tauri/` (requer pacotes `-dev` do GTK3/WebKit2GTK4.1 no Linux).
