# Próximos passos

## Entregáveis Extras

### 1. Checklist de Acesso/Clarificação

Não há segredos ou chaves de API visíveis no código que exijam rotação. O código é local-first.

* **Clarificação:** O arquivo `src-tauri/ffmpeg/ffmpeg` é um binário? O script `download-ffmpeg.sh` sugere que ele deve ser baixado. O ambiente de build precisará ter o FFmpeg configurado ou o script executado.
* **Acesso:** Se houver intenção de usar recursos de IA (sugerido pelo plugin `mcp-bridge`), será necessário configurar chaves de API locais ou remotas futuramente, mas não há config explícita no momento.

### 2. Plano Inicial para Próximos Recursos

Priorizado por Impacto/Esforço:

1. **Detecção de Duplicatas (Hashing)**
* *Esforço:* Médio (Backend Rust).
* *Impacto:* Alto. O campo `hash` já existe na tabela `images` mas não vi lógica de verificação ativa de colisão no upload/scan. Evita redundância na biblioteca.


2. **Leitura de Metadados XMP/IPTC**
* *Esforço:* Alto (Rust).
* *Impacto:* Alto. Para importar bibliotecas existentes sem perder tags ou ratings que o usuário já tenha em outros softwares (Lightroom/Bridge). O arquivo `metadata_reader.rs` parece ser o início disso.


3. **Editor de Tags em Massa (Batch Tagging)**
* *Esforço:* Baixo (Frontend).
* *Impacto:* Médio. Melhorar o `MultiInspector.tsx` para permitir adicionar/remover tags de 100+ imagens selecionadas com performance.


4. **Suporte a Vídeo Player (Preview)**
* *Esforço:* Médio (Frontend).
* *Impacto:* Médio. Atualmente foca muito em thumbnails. Permitir "tocar" o vídeo ao passar o mouse (hover scrub) como no Eagle.cool.



### 3. Status Atual (Fevereiro 2026)
*   [x] **Suporte 3D:** Pipeline completa usando Assimp CLI Bundle + ModelViewer Frontend.
    *   **Formatos verificados:** `.blend`, `.fbx`, `.obj`, `.gltf`, `.glb`, `.dae`, `.stl`, `.3ds`, `.dxf`, `.lwo`, `.lws`.
    *   **Arquitetura:** Conversão automática para GLB no cache + Ícone de fallback.

### 4. Resumo Executivo Final

Estou pronto para iniciar a implementação. O projeto possui uma arquitetura sólida separando **Tauri/Rust** (performance bruta, banco, I/O) e **SolidJS** (estado reativo, UI virtualizada). O schema de banco de dados e as stores do frontend estão bem alinhados.
**Informação útil:** Para implementar novos recursos, precisarei saber se devo priorizar a lógica no Rust (para velocidade) ou TypeScript (para flexibilidade), mas o padrão atual privilegia Rust para dados pesados e TypeScript para estado de UI, o que seguirei.

**Aguardo suas instruções para começar a codificar ou refinar alguma área específica.**