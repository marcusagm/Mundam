# PLAN: Metadata Persistence & Extraction

Implementação de persistência para Rating/Notas e visualização de metadados reais (Tamanho, Datas, EXIF).

## 1. Contexto
- **Objetivo**: Tornar o Inspector funcional exibindo dados reais do arquivo e permitindo edição de metadados do usuário.
- **Persistência**: SQLite (Tabela `images`).
- **Extração**: Tempo real (Rust Command) para EXIF/Full Metadata. Dados básicos (Size, Dates) via DB.

## 2. Breakdown de Tarefas

### Fase 1: Atualização do Schema e Banco de Dados (`src-tauri` + `db.ts`)
- [x] Verificar se `size`, `created_at`, `modified_at` estão sendo populados corretamente no indexador Rust.
- [x] Adicionar colunas `rating` (INTEGER 0-5) e `notes` (TEXT) à tabela `images` via migration ou atualização do schema inicial.
- [x] Atualizar tipos TypeScript (`ImageItem`) para incluir esses novos campos.

### Fase 2: Backend (Rust Commands)
- [x] Criar comando `get_image_metadata(path: &str)` em Rust para extrair EXIF/Metadados em tempo real. (Implementado como `get_image_exif`)
    - [x] Retorno: JSON estruturado com ISO, f-stop, Shutter Speed, Codec, Bitrate, etc.
- [x] Criar comandos de persistência:
    - [x] `update_image_rating(id: i32, rating: i32)`
    - [x] `update_image_notes(id: i32, notes: String)`

### Fase 3: Frontend - Integração (`CommonMetadata` & `AdvancedMetadata`)
- [x] **Data Fetching**: 
    - [x] No `FileInspector`, buscar os dados atualizados do DB (incluindo rating/notes).
- [x] **Rating/Notes**:
    - [x] Conectar `StarRating` e `Notes` do `CommonMetadata` às ações de update do DB.
    - [x] Implementar debounce para o campo de notas salvar automaticamente.
- [x] **AdvancedMetadata**:
    - [x] Criar componente que chama `get_image_metadata` ao carregar.
    - [x] Exibir Loading State enquanto extrai.
    - [x] Renderizar tabela de propriedades (EXIF) de forma limpa.
- [x] **Display Real**:
    - [x] Substituir os placeholders "-- MB", "--/--/--" pelos dados reais formatados.
    - [x] Implementar formatadores (`formatBytes`, `formatDate`).

## 3. Atribuição de Agentes
- **Backend**: `backend-specialist` (Rust, SQLx, Metadata Extraction).
- **Frontend**: `frontend-specialist` (Integração UI, State Management).

## 4. Critérios de Verificação
- [x] Alterar o rating de uma imagem persiste após reiniciar o app.
- [x] Notas adicionadas são salvas.
- [x] As datas de Criação/Modificação correspondem ao arquivo real (Sistema de Arquivos).
- [x] Ao expandir "Advanced Metadata", os dados EXIF reais da foto aparecem (via `get_image_exif`).

---
**Status**: Concluído ✅
