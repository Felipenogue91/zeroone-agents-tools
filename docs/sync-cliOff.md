# Extensão `cliOff` do `sync.json`

O app grava em `~/.agents/mcp/sync.json` uma chave nova para desligar um MCP **só numa CLI**:

```json
"cliOff": {
  "global": { "codex": ["chrome-devtools"] },
  "site-zeroone-linux": { "agy": ["blender"] }
}
```

- Escopo: `"global"` ou o nome da pasta do projeto (igual a `overrides`).
- Claude de projeto não usa `cliOff`: o app grava em `<projeto>/.claude/settings.local.json` `disabledMcpjsonServers` (só o Claude lê esse arquivo).
- Desligar nas três CLIs continua sendo `disabledMcpjsonServers` em `<projeto>/.claude/settings.json`.

## Compatibilidade com `~/.agents/bin/mcp-sync`

O script atual não conhece `cliOff`. Se ele rodar em modo de aplicação depois que o app usar `cliOff`, o MCP volta a aparecer na CLI desligada. O hook `mcp-sync --here` não é afetado na prática: ele só gera o `.agents/mcp_config.json` do agy da pasta atual, mas também ignora o `cliOff` do agy.

Opções (decisão do Felipe):

1. Trocar o hook e os usos do script por `zeroone-agents-tools --sync-here` / `--sync-check` / `--sync-apply`, que já entendem `cliOff` e geram a mesma saída (paridade testada).
2. Ou ensinar o script. Patch sugerido, em `with_overrides` e nos loops de geração:

```python
def cli_off(cfg, scope, cli, name):
    return name in ((cfg.get("cliOff") or {}).get(scope) or {}).get(cli, [])
```

e pular o servidor quando `cli_off(cfg, "global", "claude"|"codex"|"agy", n)` (em `plan_global`) ou `cli_off(cfg, name_do_projeto, "codex"|"agy", n)` (em `plan_project`; no Codex de projeto, gerar `enabled = false`).
