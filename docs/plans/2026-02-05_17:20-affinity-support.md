# Relatório de Implementação: Suporte Nativo a Formatos Affinity (V1 e V2)

**Data:** 2026-02-05  
**Tarefa:** Melhorar a geração de thumbnails e visualização para arquivos `.afphoto`, `.afdesign` e `.afpub`.

---

## 1. Contexto e Problema
Anteriormente, o Mundam utilizava uma estratégia de `ZipPreview` genérica para arquivos Affinity. No entanto, arquivos da versão V2 e muitos da V1 não são ZIPs válidos ou possuem containers proprietários que impediam a extração da miniatura, resultando em ícones genéricos.

## 2. Abordagem Técnica Escolhida
Foi adotada a **Extração por Assinatura Binária (Carver)**. Em vez de depender da estrutura do container (ZIP), o sistema agora escaneia o arquivo em busca de streams de dados PNG embutidos. 

Esta abordagem foi inspirada em scripts de referência (`afthumbs.py` e `png-extractor.js`) e otimizada para performance em Rust.

---

## 3. Implementações Realizadas

### A. Core de Extração (`src-tauri/src/thumbnails/affinity.rs`)
Criamos um módulo especializado para processar os arquivos binários:
- **Busca Otimizada**: O escaneamento foca nos últimos **15MB** do arquivo (onde os previews costumam ser armazenados), evitando a leitura de arquivos gigantescos (GBs) por completo.
- **Identificação de PNG**: Localiza a assinatura `\x89PNG` e mapeia até o chunk final `IEND`.
- **Seleção de Qualidade**: O algoritmo identifica todos os PNGs no range e seleciona o de **maior tamanho em bytes**, garantindo que o preview de alta resolução seja escolhido em vez de ícones pequenos.

### B. Registro de Formatos (`src-tauri/src/formats.rs`)
- Adicionada a nova estratégia `ThumbnailStrategy::Affinity`.
- Mapeamento das extensões `.afphoto`, `.afdesign` e `.afpub` para utilizarem exclusivamente este novo método.

### C. Integração no Pipeline de Thumbnails (`src-tauri/src/thumbnails/mod.rs`)
- O módulo foi exposto e integrado ao `generate_thumbnail`, permitindo que o indexador gere miniaturas WebP rápidas a partir do PNG extraído.

### D. Visualização em Alta Resolução (`src-tauri/src/protocols.rs`)
Para permitir que o usuário veja a imagem em tamanho real no `ItemView`:
- O handler do protocolo `orig://` foi atualizado para interceptar extensões Affinity.
- Em vez de servir o arquivo `.af*` bruto (que o navegador não renderiza), o backend extrai o PNG em alta resolução em tempo real e o serve como `image/png`.

### E. Frontend e Interface (`src/components/features/itemview/ItemView.tsx`)
- As extensões Affinity foram incluídas na categoria de **Imagens**.
- Isso permite que o `ImageViewer` abra esses arquivos com suporte a zoom, pan, rotação e ferramentas de inspeção, tratando-os como imagens nativas de alta qualidade.

---

## 4. Resultados Obtidos
- **Compatibilidade Total**: Funciona com Affinity V1 e V2.
- **Performance**: A geração de miniaturas é quase instantânea devido ao escaneamento seletivo no final do arquivo.
- **Experiência do Usuário**: O Mundam agora funciona como um visualizador nativo de alta fidelidade para o ecossistema Affinity, sem necessidade de softwares externos.

---
**Status:** Concluído e Validado.
