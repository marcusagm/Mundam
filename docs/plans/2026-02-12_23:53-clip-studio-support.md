# Implementação de Suporte a Arquivos Clip Studio Paint (.clip) no Mundam

Este documento detalha o processo de pesquisa, decisão arquitetural e implementação realizado para fornecer suporte a arquivos do Clip Studio Paint (`.clip`), utilizando a extração de previews de alta qualidade via banco de dados SQLite interno.

## 📋 Visão Geral

- **Objetivo**: Adicionar suporte a arquivos `.clip`, fornecendo thumbnails e previews de fidelidade total (1000x1000 pixels).
- **Data**: 12 de Fevereiro de 2026
- **Status**: Concluído ✅

---

## 🚀 Etapas da Implementação

### 1. Análise Técnica e Engenharia Reversa
Através da análise binária de arquivos de amostra e do código de referência em Dlang, identificamos que o formato `.clip` (CSFCHUNK) é um container de blocos. A descoberta chave foi:
- O preview renderizado do canvas não está em um bloco de imagem direto, mas sim dentro de um chunk chamado **`CHNKSQLi`**.
- Este chunk contém um arquivo **SQLite 3** completo.
- Dentro do SQLite, a tabela `CanvasPreview` armazena o blob `ImageData` contendo um PNG de 1000x1000 pixels.

### 2. Brainstorming de Estratégias
Exploramos três opções antes de codificar:
- **Opção A (SQLite)**: Extrair o banco, salvá-lo temporariamente e usar `sqlx` para ler o PNG. (Vencedora pela fidelidade).
- **Opção B (Scan Binário)**: Procurar magics de PNG no arquivo. (Rejeitada pelo risco de pegar miniaturas de camadas menores).
- **Opção C (Parser Nativo de Blocos Exta)**: Tentar renderizar os blocos `CHNKExta` manualmente (zlib). (Rejeitada pela complexidade extrema e necessidade de engenharia reversa pesada).

### 3. Desenvolvimento do Extrator (`clip.rs`)
Implementamos o módulo `src-tauri/src/thumbnails/extractors/clip.rs`:
- **Parser de Chunks**: Navega pela estrutura `CSFCHUNK` e `CHNKSQLi` usando leitura BigEndian para determinar offsets e comprimentos.
- **Gerenciamento de Arquivo Temporário**: Como bibliotecas SQLite/SQLx operam sobre arquivos, o chunk do banco é extraído para o diretório temporário do sistema com um nome único (`uuid`).
- **Extração via SQLx**: Uma query `SELECT ImageData FROM CanvasPreview LIMIT 1` é executada de forma assíncrona usando o runtime do Tauri.

### 4. Integração no Pipeline
- **Registry**: O formato agora está devidamente mapeado em `definitions.rs` (previamente mapeado, mas agora funcional).
- **Registry de Extratores**: O `mod.rs` de extratores foi atualizado para rotear a extensão `.clip` para o novo módulo especializado, abandonando a tentativa genérica de ZIP.

---

## 📊 Resultados e Performance (Debug Mode)

Os testes foram realizados utilizando a biblioteca de amostras fornecida. Abaixo estão os tempos de processamento observados (em modo Debug):

| Arquivo | Tamanho | Tempo de Geração |
| :--- | :--- | :--- |
| `Sketches.clip` | ~5 MB | 4.18s |
| `commission 121.clip` | ~200 MB | 8.62s |
| `01.clip` | ~220 MB | 12.61s |
| `azura 2.clip` | ~150 MB | 23.86s |

**Nota**: Em modo **Release**, espera-se que estes tempos caiam drasticamente (estimativa de 10x a 20x mais rápido), similar ao observado na otimização do XCF.

---

## 🛠️ Arquivos Modificados
- `src-tauri/src/thumbnails/extractors/clip.rs` (Novo)
- `src-tauri/src/thumbnails/extractors/mod.rs` (Registro do módulo e roteamento)
- `src-tauri/src/formats/definitions.rs` (Anatômico para suporte via NativeExtractor)

---

---

## ⏱️ Benchmarks de Geração: Debug vs. Release

Abaixo está o comparativo de tempo de geração. Os tempos em **Debug** foram coletados durante a validação inicial. Os campos de **Release** serão preenchidos após o teste de produção.

| Arquivo | Tempo (Debug) | Tempo (Release) | Ganho (Otimização) |
| :--- | :--- | :--- | :--- |
| `commission 135.clip` | 22.54s | **265ms** | **85.0x** |
| `lopunny 2.clip` | 14.09s | **169ms** | **83.3x** |
| `malo.clip` | 11.68s | **148ms** | **78.9x** |
| `01.clip` | 12.61s | **193ms** | **65.3x** |
| `commission 121.clip` | 8.62s | **220ms** | **39.1x** |
| `azura 2.clip` | 23.86s | **735ms** | **32.4x** |
| `lopunny5.clip` | 10.96s | **343ms** | **31.9x** |
| `lopunny 1.clip` | 10.17s | **334ms** | **30.4x** |
| `commission 125.clip` | 12.07s | **427ms** | **28.2x** |
| `commission 90.clip` | 11.44s | **439ms** | **26.0x** |
| `Sketches.clip` | 4.18s | **236ms** | **17.7x** |
| `commission 146.clip` | 12.97s | **746ms** | **17.3x** |

### Conclusão do Benchmark

O ganho de performance em modo **Release** foi colossal, com otimizações variando de **17x a 85x**.

A implementação nativa em Rust, ao operar com otimizações de produção, consegue processar arquivos de centenas de megabytes (extraindo o banco SQLite, consultando o blob e redimensionando a imagem) em **menos de 1 segundo**. 

Isso confirma que, apesar da complexidade de extrair o SQLite interno para um arquivo temporário, a infraestrutura do Mundam é extremamente eficiente, eliminando completamente o gargalo que existia no modo Debug e fornecendo uma experiência de catálogo fluida para artistas que utilizam o Clip Studio Paint.
