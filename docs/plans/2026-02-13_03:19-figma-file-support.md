# Implementação de Suporte a Arquivos Figma (.fig) no Mundam

Este documento detalha o plano de implementação para fornecer suporte a arquivos do Figma (`.fig`), permitindo a geração de thumbnails e previews através do processamento do formato de arquivo local.

## 📋 Visão Geral

- **Objetivo**: Adicionar suporte a arquivos `.fig`, fornecendo thumbnails e previews de alta fidelidade para artistas e designers.
- **Data**: 13 de Fevereiro de 2026
- **Status**: Fase 1 Concluída ✅ (Fase 2 e 3 Planejadas)

---

## 🚀 Fase 1: Thumbnails e Previews Básicos (Atual)

O objetivo desta fase é garantir que arquivos `.fig` sejam visíveis na biblioteca com sua aparência oficial.

### Estratégia Técnica
Arquivos `.fig` exportados via "Save local copy" são arquivos ZIP. Eles contêm um `thumbnail.png` na raiz que representa o preview oficial do projeto.

### Passos de Implementação
1.  **Registro do Formato**:
    - Atualizar `src-tauri/src/formats/definitions.rs` para alterar a estratégia do formato Figma de `Icon` para `NativeExtractor`.
    - Definir `PreviewStrategy::NativeExtractor`.
2.  **Desenvolvimento do Extrator**:
    - Modificar `src-tauri/src/thumbnails/extractors/mod.rs` para incluir a lógica de extração do `thumbnail.png` de dentro do ZIP do Figma.
3.  **Validação**:
    - Testar com arquivos de exemplo da comunidade para garantir que o thumbnail oficial seja extraído corretamente.

---

## 📅 Fase 2: Metadados e Informações do Projeto (Futuro)

Enriquecer o painel do Inspector com dados reais do arquivo Figma.

### Passos de Implementação
1.  **Parsing de `meta.json`**:
    - Implementar a leitura do arquivo `meta.json` dentro do ZIP.
    - Extrair o nome original do arquivo (`file_name`), data de exportação (`exported_at`) e cor de fundo (`background_color`).
2.  **Exibição no Frontend**:
    - Atualizar o componente de metadados para mostrar estas informações específicas do Figma quando o arquivo for selecionado.

---

## 📅 Fase 3: Extração de Ativos/Imagens (Futuro)

Permitir que o Mundam funcione como um visualizador de ativos para o projeto Figma.

### Passos de Implementação
1.  **Scanner de Galeria**:
    - Criar uma nova aba no Previewer do Mundam chamada "Assets".
    - Implementar uma função que lista e extrai as imagens do diretório `images/` dentro do arquivo `.fig`.
2.  **Navegação de Layers (Opcional/Experimental)**:
    - Integrar lógica de decodificação Kiwi para ler o `canvas.fig` e extrair nomes de páginas ou frames, permitindo uma navegação estruturada básica.

---

## 🛠️ Arquivos Modificados (Fase 1)
- `src-tauri/src/formats/definitions.rs`: Configuração do formato.
- `src-tauri/src/thumbnails/extractors/mod.rs`: Lógica de extração centralizada.

---

## ✅ Critérios de Aceite
- [ ] Arquivos `.fig` mostram o thumbnail oficial na galeria.
- [ ] O modo "Preview" (tela cheia) mostra a imagem de thumbnail extraída.
- [ ] O sistema não falha ao encontrar arquivos `.fig` que não sejam ZIPs (fallback para ícone).
