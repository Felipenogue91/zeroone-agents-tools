# ZeroOne Agents Tools

Fork aberto (licença MIT) do `tylergraydev/claude-code-tool-manager` mantido pela ZeroOne para uso próprio e distribuição pública multi-usuário em **Linux** (`.deb`, `.AppImage`, `.rpm`) e **Windows** (`.exe`, `.msi`).

## Regras para multi-usuário e cross-platform

- **Nunca hardcodar caminhos pessoais** (como `/home/<usuario>/...` ou caminhos Windows fixos), tokens, credenciais ou dados de máquina específica no código ou em arquivos versionados.
- Resolver todos os diretórios de usuário e configuração de forma cross-platform (`dirs::home_dir()`, `std::path::PathBuf`, variáveis de ambiente padrão) compatível com Linux e Windows.
- Recursos opcionais (como `.agents/skills` e Antigravity CLI) devem ser detectados apenas se existirem na máquina do usuário, sem erro nem ruído quando ausentes.

## Identidade visual (somente leitura)

- Referência oficial da marca (não fundir repositórios):
  - `site-zeroone-linux/zeroone-brand-kit/`
  - `site-zeroone-linux/src/styles/global.css`
- Os assets aprovados já estão copiados dentro deste repositório (`static/brand/`, `static/favicon.png`, `src-tauri/icons/`).

## Stack e comandos de verificação

- **Frontend:** Svelte 5 + SvelteKit + Tailwind CSS v4
  - `npm run test:run` — suíte Vitest (baseline `v3.10.0`: 84 arquivos / 1567 testes passando)
  - `npm run build` — build estático de produção (`build/`)
  - `npm run check` — `svelte-check` possui 11 erros TS pré-existentes do upstream em `src/lib/components/containers/*` e `src/lib/components/subagents/AgentMemoryPanel.svelte`; não introduzir novos erros nos arquivos modificados.
- **Backend nativo:** Tauri 2 + Rust (`src-tauri/`)
  - `cargo test` dentro de `src-tauri/` (requer pacotes `-dev` do GTK3/WebKit2GTK4.1 no Linux).

## Contexto de design (veio do CLAUDE.md do projeto original)

### Users
Developers of all experience levels who manage Claude Code and other AI coding assistant configurations. They want a clear, approachable interface that makes complex configuration feel simple. The primary job: see everything at a glance, toggle tools on/off, and keep configs in sync — without hand-editing JSON files.

### Brand Personality
**Friendly, capable, modern.** Think Notion or Figma — approachable but clearly powerful. Never intimidating, never dumbed down.

### Aesthetic Direction
- **Visual tone:** Clean, information-rich, native-feeling. Inspired by VS Code's settings UI — dense but organized, functional but not ugly.
- **Theme:** Dark-first (class-based toggle), blue primary palette (Tailwind blue scale), gray neutrals, system-ui font stack.
- **Anti-references:** Must never feel like Electron bloat — no sluggish animations, no web-page-pretending-to-be-native energy. Should feel fast and lightweight like a native app.
- **Icons:** Lucide icon set throughout.
- **Layout:** Sidebar navigation + main content area. Card-based UI with rounded corners (`rounded-xl`), subtle shadows, custom scrollbars.

### Design Principles

1. **Clarity over cleverness** — Every element should be immediately understandable. Prefer familiar patterns (toggles, cards, lists) over novel interactions. Labels should be specific and jargon-free where possible.

2. **Density with breathing room** — Show lots of information without feeling cramped. Use consistent spacing, clear section boundaries, and visual hierarchy to keep dense layouts scannable.

3. **Native performance feel** — Interactions should feel instant. Avoid heavy animations. Transitions should be subtle and functional (hover states, focus rings), not decorative. The app should feel as fast as the terminal it configures.

4. **Dark mode first, light mode supported** — Design in dark mode, ensure light mode works well. The existing gray-900 background with gray-800 cards and blue accents is the foundation.

5. **Accessible by default** — Good contrast ratios, keyboard navigable, semantic HTML. Focus indicators on all interactive elements. No information conveyed by color alone.
