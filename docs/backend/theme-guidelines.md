# Theme Guidelines

Este documento define o padrão para temas do Mundam: temas embutidos (CSS) e temas customizados pelo usuário (JSON). O objetivo é que cada aspecto da aparência (cor, tipografia, geometria) tenha um único dono, evitando conflitos.

## Camadas da aparência

| Aspecto               | Controlado por         | Onde é aplicado                                                                   | Tokens                              |
| --------------------- | ---------------------- | --------------------------------------------------------------------------------- | ----------------------------------- |
| Modo (`dark`/`light`) | `appearance_mode`      | atributo `data-theme` em `<html>`                                                 | semânticos de cor (`tokens.css`)    |
| Accent                | `appearance_theme`     | classe `html.theme-*` ([themes/index.css](../../src/styles/themes/index.css))     | **apenas** `--theme-h`, `--theme-c` |
| Tipografia            | `appearance_font_size` | classe `html.font-size-*` ([themes/index.css](../../src/styles/themes/index.css)) | **apenas** `--p-font-size-*`        |
| Geometria             | `appearance_radius`    | estilo inline em `<html>`                                                         | **apenas** `--radius-*`             |
| Cores customizadas    | tema `custom`          | estilo inline em `<html>`                                                         | **apenas** a whitelist abaixo       |

Regra de ouro: **um tema de cor nunca define tipografia, espaçamento ou geometria**, e um preset de tipografia/geometria nunca define cores.

## Temas embutidos (CSS)

Ficam em `src/styles/themes/index.css`, importado por `tokens.css`.

- Presets de accent: `html.theme-<nome> { --theme-h: …; --theme-c: …; }`. Nada além disso.
- Presets de tamanho de fonte: `html.font-size-<nome> { --p-font-size-*: …; }`.
- Use o seletor `html.<classe>` para vencer a especificidade de `:root`.
- Não adicione regras de cor semântica (`--bg-*`, `--text-*`…) nesse arquivo; elas pertencem a `tokens.css` (`:root` para dark, `[data-theme="light"]` para light).

## Temas customizados (JSON)

O usuário cria, edita, duplica, exclui, importa e exporta temas em **Settings → Appearance → Interface Mode → Custom**. Não é necessário conhecer CSS.

### Formato do arquivo

Extensão sugerida: `.mundam-theme.json`.

```json
{
  "schema": "mundam-theme",
  "version": 1,
  "name": "Ocean",
  "base": "dark",
  "colors": {
    "--bg-page": "#0b1620",
    "--bg-surface-1": "#10202e",
    "--text-primary": "#dbe9f4",
    "--action-primary-bg": "#2a9df4"
  }
}
```

| Campo     | Tipo                  | Descrição                                                                            |
| --------- | --------------------- | ------------------------------------------------------------------------------------ |
| `schema`  | `"mundam-theme"`      | Obrigatório. Identifica o formato.                                                   |
| `version` | inteiro ≥ 1           | Versão do formato (atual: `1`). Arquivos de versão maior são rejeitados.             |
| `name`    | string (1–60)         | Nome exibido no select de temas.                                                     |
| `base`    | `"dark"` \| `"light"` | Modo cujos tokens servem de fallback. Define `data-theme`, `color-scheme` e sombras. |
| `colors`  | objeto                | Mapa `token → cor`. Todos os tokens são opcionais; os ausentes herdam da `base`.     |

### Tokens permitidos

Somente os tokens semânticos de cor abaixo são aceitos (fonte da verdade: `THEME_TOKEN_GROUPS` em [customTheme.ts](../../src/core/theme/customTheme.ts)). Qualquer outra chave é descartada na importação.

- **Backgrounds:** `--bg-page`, `--bg-sidebar`, `--bg-header`, `--bg-surface-1`, `--bg-surface-2`, `--bg-surface-3`, `--bg-surface-active`
- **Borders:** `--border-subtle`, `--border-default`, `--border-strong`, `--border-active`
- **Text:** `--text-primary`, `--text-secondary`, `--text-tertiary`, `--text-inverse`
- **Actions:** `--action-primary-bg`, `--action-primary-bg-hover`, `--action-primary-text`, `--action-secondary-bg`, `--action-secondary-bg-hover`, `--action-secondary-text`, `--action-destructive-bg`, `--action-destructive-text`
- **Status:** `--text-success`, `--text-warning`, `--text-destructive`, `--text-info`, `--bg-success`, `--bg-warning`, `--bg-error`, `--bg-info`

Tokens primitivos (`--p-*`), tipografia, espaçamento, `--radius-*`, sombras e transições **não** são customizáveis por tema.

### Valores permitidos

Cores em `#hex` (3–8 dígitos), `rgb()/rgba()`, `hsl()/hsla()`, `oklch()`, `oklab()`, `lab()`, `lch()` ou `color()`, até 100 caracteres. Valores com `url()`, `var()`, `;`, `{`, `}` ou qualquer outro conteúdo são descartados, garantindo que um arquivo importado não injete CSS arbitrário.

### Comportamento

- O tema ativo é aplicado como estilo inline em `<html>` (vence `:root` e `[data-theme]`); ao trocar de modo, as propriedades anteriores são removidas.
- O accent (`html.theme-*`) continua valendo para tokens não sobrescritos que derivam de `--theme-h/--theme-c`.
- Tipografia e geometria continuam independentes e valem para qualquer tema.
- Importar sempre gera um novo `id` interno; nunca sobrescreve um tema existente.
- Se o tema ativo for excluído, o modo volta para `system`.

## Persistência

Via `tauriService.setSetting`:

| Chave                        | Conteúdo                                   |
| ---------------------------- | ------------------------------------------ |
| `appearance_mode`            | `dark` \| `light` \| `system` \| `custom`  |
| `appearance_custom_theme_id` | id do tema customizado ativo               |
| `appearance_custom_themes`   | JSON array de `{ id, name, base, colors }` |

A sincronização entre janelas usa o evento `appearance:sync`.

## Adicionando novos tokens customizáveis

1. Garanta que o token existe em `tokens.css` nos blocos dark (`:root`) e light (`[data-theme="light"]`).
2. Adicione-o a `THEME_TOKEN_GROUPS` em `src/core/theme/customTheme.ts` (a UI e a validação o incluem automaticamente).
3. Tokens opacos apenas: o editor usa seletor de cor hexadecimal. Para tokens com alpha, estenda o editor antes de incluí-los.
4. Se o formato do arquivo mudar de forma incompatível, incremente `THEME_FILE_VERSION`.

## Arquivos relevantes

- [appearanceStore.ts](../../src/core/store/appearanceStore.ts): estado, persistência e `apply()`.
- [customTheme.ts](../../src/core/theme/customTheme.ts): schema, whitelist, validação, import/export.
- [applyCustomTheme.ts](../../src/core/theme/applyCustomTheme.ts): resolve o tema ativo e aplica as cores no `<html>`.
- [color.ts](../../src/utils/color.ts): `isSafeCssColor` e `convertCssColorToHexadecimal` (utilitários genéricos de cor).
- [CustomThemeEditor.tsx](../../src/components/features/settings/CustomThemeEditor.tsx): seleção, import/export e exclusão.
- [CustomThemeColorFields.tsx](../../src/components/features/settings/CustomThemeColorFields.tsx): edição de nome, base e cores.
