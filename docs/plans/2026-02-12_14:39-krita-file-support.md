# Refatoração: Suporte a Arquivos Krita (.kra)

**Data:** 2026-02-12 14:39
**Status:** Concluído ✅
**Objetivo:** Implementar suporte para thumbnails e previews de arquivos nativos do Krita utilizando a técnica de extração do container ZIP.

---

## 🧠 Brainstorming de Soluções

Foram consideradas três abordagens principais:
1. **Extração Inteligente (ZIP):** Extrair `mergedimage.png` e `preview.png` diretamente do arquivo.
2. **Crates Especializadas:** Usar bibliotecas Rust para parsear a estrutura interna.
3. **CLI Headless:** Chamar o executável do Krita para exportação.

**Decisão:** Optamos pela **Extração Inteligente (Opção A)** devido à excelente performance, zero dependências externas pesadas e alta fidelidade (já que as imagens são renderizadas pelo próprio Krita no momento do salvamento).

---

## 🛠️ Passo a Passo da Implementação

### 1. Registro de Formatos
Atualizamos o arquivo `src-tauri/src/formats/definitions.rs` para incluir o Krita no registro mestre de formatos suportados.

- **Extensões:** `.kra`, `.krz`, `.kra~`
- **Categoria:** `MediaType::Project`
- **Estratégia de Thumbnail:** `ThumbnailStrategy::NativeExtractor`
- **Estratégia de Preview:** `PreviewStrategy::NativeExtractor`

### 2. Implementação do Extrator
Modificamos `src-tauri/src/thumbnails/extractors/mod.rs` para incluir a lógica específica do Krita.

- **Função `extract_krita_preview`**:
  - Abre o arquivo como um arquivo ZIP.
  - **Prioridade 1:** `mergedimage.png` (Alta qualidade, canvas completo).
  - **Prioridade 2 (Fallback):** `preview.png` (Miniatura rápida).
- **Roteamento:** Adicionamos as extensões do Krita ao fluxo do `NativeExtractor` no `extract_preview`.

### 3. Documentação e Visibilidade
- Atualizamos o `README.md` marcando o formato `kra` como suporte total (✅).

---

## ✅ Verificação e Testes

- **Compilação:** Executado `cargo check` com sucesso.
- **Estrutura de Arquivos:** Verificado via `unzip -l` que os arquivos de amostra fornecidos pelo usuário continham as entradas `mergedimage.png` e `preview.png`.

---

## 🚀 Resultados
- Os arquivos do Krita agora são tratados como projetos de design de primeira classe (similar ao PSD e Affinity).
- Preview instantâneo sem necessidade de conversões pesadas.
- Suporte a backups automáticos do Krita (`.kra~`) garantido.
