Este é um plano de refatoração focado em **Robustez, Type Safety e Single Source of Truth** (Fonte Única da Verdade).

O problema atual é a duplicidade de conhecimento (extensões no Frontend vs. lógica no Backend) e a fragilidade da detecção por string. A solução propõe uma arquitetura onde o **Rust** detém a definição canônica dos formatos, e o Frontend consome isso dinamicamente ou estaticamente, enquanto a detecção de arquivos migra de "Extensão" para "Magic Bytes" (conteúdo real do arquivo).

---

# Plano de Refatoração: Unified Media Detection System (UMDS)

## Visão Geral da Arquitetura

1. **Domain Layer (Rust):** Criação de um registro central (`FormatRegistry`) que define quais MIME types, extensões e "Estratégias de Thumbnail" o sistema suporta.
2. **Detection Layer (Rust):** Implementação de uma *crate* de detecção (`infer`) para identificar arquivos pelo cabeçalho binário (Magic Bytes), usando a extensão apenas como fallback.
3. **Contract Layer (Bridge):** Exposição dessa configuração para o Frontend via comando Tauri, eliminando o `fileFormats.json` hardcoded.

---

## Etapa 1 — Dependências e Estrutura de Dados (Rust)

Precisamos adicionar bibliotecas para detecção real de arquivos e serialização.

**Ação 1.1:** Adicionar crates ao `src-tauri/Cargo.toml`.

```toml
[dependencies]
# ... outras deps
infer = "0.15"       # Para detecção de MIME types via magic bytes
mime_guess = "2.0"   # Fallback para detecção via extensão e lookup reverso
serde = { version = "1.0", features = ["derive"] } # Para enviar configs ao front
strum = "0.26"       # Para iterar sobre Enums facilmente
strum_macros = "0.26"

```

**Ação 1.2:** Criar o módulo `src-tauri/src/formats.rs`.
Este arquivo será a **Fonte Única da Verdade**.

```rust
// src-tauri/src/formats.rs
use serde::Serialize;
use strum_macros::{EnumIter, Display};

#[derive(Debug, Clone, Serialize, EnumIter, Display, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum MediaType {
    Image,
    Video,
    Audio,
    Project, // ex: .psd, .ai
    Archive, // ex: .zip
    Model3D,
    Font,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub enum ThumbnailStrategy {
    NativeImage, // Decodificadores Rust (zune-jpeg, image-rs)
    Ffmpeg,      // Vídeo e formatos complexos
    Webview,     // SVG, HTML
    Icon,        // Fallback para fontes/arquivos sem preview
    None,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileFormat {
    pub name: &'static str,
    pub extensions: &'static [&'static str],
    pub mime_types: &'static [&'static str],
    pub type_category: MediaType,
    #[serde(skip)] // O frontend não precisa saber a estratégia interna de render
    pub strategy: ThumbnailStrategy,
}

// O REGISTRO MESTRE (Substitui o match gigante e o JSON do frontend)
pub const SUPPORTED_FORMATS: &[FileFormat] = &[
    FileFormat {
        name: "JPEG Image",
        extensions: &["jpg", "jpeg", "jpe"],
        mime_types: &["image/jpeg"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::NativeImage,
    },
    FileFormat {
        name: "Portable Network Graphics",
        extensions: &["png"],
        mime_types: &["image/png"],
        type_category: MediaType::Image,
        strategy: ThumbnailStrategy::NativeImage,
    },
    FileFormat {
        name: "Photoshop Document",
        extensions: &["psd", "psb"],
        mime_types: &["image/vnd.adobe.photoshop"],
        type_category: MediaType::Project,
        strategy: ThumbnailStrategy::Ffmpeg, // Ou imagem crate com features específicas
    },
    // ... Adicionar todos os outros formatos aqui
];

```

---

## Etapa 2 — Lógica de Detecção Inteligente

Aqui substituímos o `match` de string por uma lógica robusta.

**Ação 2.1:** Implementar o resolvedor em `src-tauri/src/formats.rs`.

```rust
use std::path::Path;
use std::fs::File;
use std::io::Read;

impl FileFormat {
    /// Tenta detectar o formato real do arquivo.
    /// Prioridade: 1. Magic Bytes (infer) -> 2. Extensão (mime_guess) -> 3. Unknown
    pub fn detect(path: &Path) -> Option<&'static FileFormat> {
        // 1. Tentar ler os primeiros bytes (Header)
        // Isso evita ler o arquivo todo, garantindo performance
        let mut buffer = [0u8; 8192]; // 8KB é suficiente para maioria dos magic bytes
        if let Ok(mut file) = File::open(path) {
            if file.read(&mut buffer).is_ok() {
                if let Some(kind) = infer::get(&buffer) {
                    // Busca no nosso registro pelo MIME retornado pelo infer
                    if let Some(fmt) = SUPPORTED_FORMATS.iter().find(|f| f.mime_types.contains(&kind.mime_type())) {
                        return Some(fmt);
                    }
                }
            }
        }

        // 2. Fallback: Extensão (caso o arquivo esteja bloqueado ou seja texto/json/svg que infer falha as vezes)
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext_lower = ext.to_lowercase();
            return SUPPORTED_FORMATS.iter().find(|f| f.extensions.contains(&ext_lower.as_str()));
        }

        None
    }
}

```

---

## Etapa 3 — Refatoração do Consumidor (`thumbnails.rs`)

O arquivo `src-tauri/src/thumbnails.rs` agora fica extremamente limpo e desacoplado.

**Ação 3.1:** Atualizar `get_strategy` (ou renomear para `resolve_thumbnail_task`).

```rust
// src-tauri/src/thumbnails.rs
use crate::formats::{FileFormat, ThumbnailStrategy};

pub fn get_thumbnail_strategy(path: &Path) -> ThumbnailStrategy {
    match FileFormat::detect(path) {
        Some(format) => format.strategy.clone(),
        None => ThumbnailStrategy::Icon, // Arquivo desconhecido recebe ícone genérico
    }
}

```

**Impacto de Performance:**

* A leitura de 8KB de header é insignificante (nanossegundos em SSD NVMe) comparada à decodificação da imagem completa.
* Garante que se alguém renomear `virus.exe` para `image.jpg`, o sistema detectará que NÃO é um JPEG e negará o processamento (segurança).

---

## Etapa 4 — Ponte com o Frontend (A "Single Source of Truth")

Para eliminar o `fileFormats.json` no TypeScript, o Backend deve fornecer a configuração na inicialização.

**Ação 4.1:** Criar um comando Tauri para expor os formatos.

```rust
// src-tauri/src/lib.rs ou um novo command file
#[tauri::command]
pub fn get_supported_formats() -> Vec<formats::FileFormat> {
    formats::SUPPORTED_FORMATS.to_vec()
}

```

**Ação 4.2:** No Frontend (`src/core/store/systemStore.ts` ou similar), carregar essa config.

```typescript
// src/core/config/formats.ts
import { invoke } from "@tauri-apps/api/core";

export interface FileFormat {
    name: string;
    extensions: string[];
    mimeTypes: string[];
    typeCategory: 'Image' | 'Video' | 'Audio' | 'Project' | 'Archive' | 'Model3D' | 'Font' | 'Unknown';
}

// Cache local para acesso síncrono rápido após boot
let formatsCache: FileFormat[] = [];

export const loadFormats = async () => {
    formatsCache = await invoke<FileFormat[]>('get_supported_formats');
    console.log(`[System] Loaded ${formatsCache.length} supported formats from Backend.`);
};

export const getExtensionsByCategory = (category: string) => {
    return formatsCache
        .filter(f => f.typeCategory === category)
        .flatMap(f => f.extensions);
};

```

---

## Resumo dos Ganhos

| Aspecto | Antes (Atual) | Depois (Refatorado) |
| --- | --- | --- |
| **Manutenção** | Adicionar um formato exige editar Rust (`match`) e TS (`json`). | Edita-se apenas `formats.rs` no Rust. O resto se adapta. |
| **Robustez** | Confia cegamente na extensão (`.jpg`). | Verifica assinatura binária (Magic Bytes). Mais seguro. |
| **Código** | `match` gigante e difícil de ler (Code Smell). | Iteração limpa sobre Structs tipadas. |
| **Frontend** | Lista manual estática desatualizada. | Sincronizado dinamicamente com as capacidades reais do Backend. |

Deseja que eu gere o código completo do arquivo `formats.rs` com uma lista inicial de formatos populares (JPG, PNG, WEBP, MP4, MKV, PDF, etc.) para você copiar e colar?